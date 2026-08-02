//! User keybind customizations, and where they live on disk.
//!
//! # Why the key is a command id
//!
//! Overrides are stored as `command_id -> chords`, never `action -> chords`.
//! An action enum is a compile-time artifact of one build: reorder it, rename
//! a variant, or split a scope, and every stored binding silently points at
//! the wrong thing. A command id is a string the author chose and is expected
//! to keep stable, so an unknown id at load time is *ignorable* — the binding
//! just falls back to its default — rather than actively wrong.
//!
//! # Why the app name is a parameter
//!
//! tempera does not know it is running dawai. [`SavedKeybinds::load`] takes an
//! app name and derives `<config dir>/<app>/keybinds.json` from it, so two
//! applications built on tempera do not fight over one file.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::chord::SerializedChord;

/// Overrides for one scope: `command_id -> the chords bound to it`.
///
/// An empty vec is meaningful — it records "the user unbound this", which is
/// different from "the user never touched it" (an absent key).
pub type ScopeOverrides = HashMap<String, Vec<SerializedChord>>;

/// Every user keybind override, grouped by scope.
///
/// Anything absent falls back to the binding declared at registration.
///
/// The scope key is an open string rather than a fixed set of fields. The
/// original implementation had seven named struct fields (`global`,
/// `timeline`, `piano_roll`, …), which meant this type — and its file format —
/// had to change whenever the application grew a new scope. That is precisely
/// the coupling tempera exists to remove.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SavedKeybinds {
    pub scopes: HashMap<String, ScopeOverrides>,
}

impl SavedKeybinds {
    /// Overrides for one scope, or an empty map if the scope is untouched.
    pub fn scope(&self, scope: &str) -> Option<&ScopeOverrides> {
        self.scopes.get(scope)
    }

    /// Record an override. Pass an empty `chords` to record "unbound".
    pub fn set(&mut self, scope: &str, command_id: &str, chords: Vec<SerializedChord>) {
        self.scopes
            .entry(scope.to_owned())
            .or_default()
            .insert(command_id.to_owned(), chords);
    }

    /// Forget an override, restoring the registered default.
    pub fn clear(&mut self, scope: &str, command_id: &str) {
        if let Some(overrides) = self.scopes.get_mut(scope) {
            overrides.remove(command_id);
        }
    }

    /// Read overrides for `app_name`, or defaults if the file is missing or
    /// unreadable.
    ///
    /// A corrupt file is logged and ignored rather than propagated: losing
    /// custom keybinds is a far better failure than refusing to start.
    pub fn load(app_name: &str) -> Self {
        let path = Self::storage_path(app_name);
        let Ok(bytes) = std::fs::read(&path) else {
            return Self::default();
        };
        match serde_json::from_slice(&bytes) {
            Ok(saved) => saved,
            Err(e) => {
                warn!(
                    "[keybinds] failed to parse {}: {e}. Falling back to defaults.",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Write overrides for `app_name`. Failures are logged, not propagated.
    pub fn save(&self, app_name: &str) {
        let path = Self::storage_path(app_name);
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            warn!(
                "[keybinds] failed to create config dir {}: {e}",
                parent.display()
            );
            return;
        }
        match serde_json::to_vec_pretty(self) {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&path, bytes) {
                    warn!("[keybinds] failed to write {}: {e}", path.display());
                }
            }
            Err(e) => warn!("[keybinds] failed to serialize: {e}"),
        }
    }

    /// `<config dir>/<app_name>/keybinds.json`, falling back to the working
    /// directory on platforms with no config dir.
    pub fn storage_path(app_name: &str) -> PathBuf {
        dirs::config_dir()
            .map(|d| d.join(app_name).join("keybinds.json"))
            .unwrap_or_else(|| PathBuf::from("keybinds.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_then_read_back() {
        let mut saved = SavedKeybinds::default();
        saved.set("global", "edit.undo", vec![vec!["Cmd".into(), "Z".into()]]);

        let scope = saved.scope("global").expect("scope present");
        assert_eq!(scope["edit.undo"], vec![vec!["Cmd", "Z"]]);
    }

    #[test]
    fn unbound_and_untouched_are_different() {
        let mut saved = SavedKeybinds::default();
        saved.set("global", "edit.undo", vec![]);

        // Explicitly unbound: present, but with no chords.
        assert_eq!(
            saved.scope("global").unwrap().get("edit.undo"),
            Some(&vec![])
        );
        // Never touched: absent, so the registered default applies.
        assert_eq!(saved.scope("global").unwrap().get("edit.redo"), None);
    }

    #[test]
    fn clear_restores_the_default() {
        let mut saved = SavedKeybinds::default();
        saved.set("global", "edit.undo", vec![vec!["Cmd".into(), "Y".into()]]);
        saved.clear("global", "edit.undo");
        assert!(saved.scope("global").unwrap().get("edit.undo").is_none());
    }

    #[test]
    fn json_shape_is_scope_keyed() {
        let mut saved = SavedKeybinds::default();
        saved.set("global", "edit.undo", vec![vec!["Cmd".into(), "Z".into()]]);

        let json = serde_json::to_string(&saved).unwrap();
        // `#[serde(transparent)]` means the map is the document — no wrapper
        // object — so the file reads as {scope: {command: [[key,…]]}}.
        assert_eq!(json, r#"{"global":{"edit.undo":[["Cmd","Z"]]}}"#);

        let back: SavedKeybinds = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.scope("global").unwrap()["edit.undo"],
            vec![vec!["Cmd", "Z"]]
        );
    }

    #[test]
    fn storage_path_is_namespaced_by_app() {
        let a = SavedKeybinds::storage_path("dawai");
        let b = SavedKeybinds::storage_path("other");
        assert_ne!(a, b, "two tempera apps must not share a keybinds file");
        assert!(a.ends_with("dawai/keybinds.json"));
    }
}
