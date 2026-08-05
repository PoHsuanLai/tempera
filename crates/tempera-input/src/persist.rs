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
//! # The host owns the path
//!
//! [`SavedKeybinds::load`] and [`SavedKeybinds::save`] take a [`Path`]. They
//! do no path policy of their own — where a file lives is the application's
//! decision, not a widget library's.
//!
//! [`SavedKeybinds::storage_path`] is still here and still derives
//! `<config dir>/<app>/keybinds.json`, because that convention is a genuinely
//! useful default and two tempera apps must not fight over one file. The
//! difference is that it is now a *helper the host may call* rather than a
//! rule the library applies. A host that wants a portable install, an
//! `--config` flag, or a test that writes somewhere harmless can simply pass a
//! different path.
//!
//! That last case is what forced this. When the path was derived internally
//! there was no parameter to redirect, so a downstream test suite could not
//! isolate itself: `dawai-shell` had to route around it by overriding the app
//! *name* per test, which leaks a stray config directory per test process. The
//! alternative was overriding `XDG_CONFIG_HOME`, and `std::env::set_var` is
//! `unsafe` and unsound under the default multi-threaded test harness. A
//! library that can only be tested by mutating global process state, or by
//! writing to the developer's real home directory, has made that choice for
//! everyone downstream.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

    /// Read overrides from `path`, or defaults if the file is missing or
    /// unreadable.
    ///
    /// A corrupt file is logged and ignored rather than propagated: losing
    /// custom keybinds is a far better failure than refusing to start.
    pub fn load(path: &Path) -> Self {
        let Ok(bytes) = std::fs::read(path) else {
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

    /// Write overrides to `path`. Failures are logged, not propagated.
    pub fn save(&self, path: &Path) {
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
                if let Err(e) = std::fs::write(path, bytes) {
                    warn!("[keybinds] failed to write {}: {e}", path.display());
                }
            }
            Err(e) => warn!("[keybinds] failed to serialize: {e}"),
        }
    }

    /// `<config dir>/<app_name>/keybinds.json`, falling back to the working
    /// directory on platforms with no config dir.
    ///
    /// A convenience for hosts that want the conventional location. Nothing in
    /// tempera calls this on a host's behalf — see the module docs.
    pub fn storage_path(app_name: &str) -> PathBuf {
        dirs::config_dir()
            .map(|d| d.join(app_name).join("keybinds.json"))
            .unwrap_or_else(|| PathBuf::from("keybinds.json"))
    }
}

/// A keybinds path under the system temp directory, unique to this process.
///
/// For tests that build the plugin but do not care about persistence, which is
/// most of them. Nothing is created — [`SavedKeybinds::load`] treats a missing
/// file as defaults, and a test that never rebinds never writes.
///
/// Exported rather than duplicated per test module because the failure it
/// prevents is silent: a test that omits it reads, and may overwrite, the
/// developer's own keybinds. Every such test previously passed a deliberately
/// unused app name (`"tempera-test-unused"`) to stay clear of the real file —
/// a convention that worked only as long as everyone remembered it.
pub fn scratch_keybinds_path() -> PathBuf {
    std::env::temp_dir()
        .join(format!("tempera-test-{}", std::process::id()))
        .join("keybinds.json")
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

    /// A directory that removes itself, so these tests need no dev-dependency.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("tempera-persist-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("keybinds.json")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_saved_file_is_written_where_it_was_asked_to_be() {
        // The property the app-name API could not offer: the caller names the
        // destination. Previously the path was derived internally, so this was
        // unobservable and a test could only assert against the developer's
        // real config directory.
        let scratch = Scratch::new("explicit");
        let mut saved = SavedKeybinds::default();
        saved.set("global", "edit.undo", vec![vec!["Cmd".into(), "Z".into()]]);

        saved.save(&scratch.file());

        assert!(
            scratch.file().exists(),
            "save did not write to the path it was given"
        );
        let back = SavedKeybinds::load(&scratch.file());
        assert_eq!(
            back.scope("global").unwrap()["edit.undo"],
            vec![vec!["Cmd", "Z"]]
        );
    }

    #[test]
    fn saving_creates_the_directory() {
        // The first save on a clean machine — the case that matters most —
        // has no config directory yet, and `fs::write` will not make one.
        let scratch = Scratch::new("mkdir");
        assert!(!scratch.0.exists());

        SavedKeybinds::default().save(&scratch.file());

        assert!(scratch.file().exists());
    }

    #[test]
    fn a_missing_file_loads_as_defaults() {
        let scratch = Scratch::new("missing");
        assert!(SavedKeybinds::load(&scratch.file()).scopes.is_empty());
    }

    #[test]
    fn a_corrupt_file_loads_as_defaults_rather_than_failing() {
        // Losing custom keybinds beats refusing to start.
        let scratch = Scratch::new("corrupt");
        std::fs::create_dir_all(&scratch.0).expect("mkdir");
        std::fs::write(scratch.file(), "{ not json").expect("write");

        assert!(SavedKeybinds::load(&scratch.file()).scopes.is_empty());
    }
}
