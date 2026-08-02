//! What a host declares to add a tab.
//!
//! # Why a registry and not an enum
//!
//! The implementation this replaces had a closed six-variant `SettingsTab`
//! enum, and adding a tab meant editing **four places that had to stay in
//! lockstep**: the enum, a hardcoded `TABS` label table, a `TabBodies`
//! `SystemParam` with six named `Query` fields, and two hand-written
//! six-tuples passed to the visibility system. Miss one and the failure is
//! silent — the six-way `let (Ok(a), Ok(b), …)` destructure meant a single
//! missing body disabled switching for *every* tab.
//!
//! It also could not work at all here. A crate that names `Audio` and
//! `Extensions` is a crate that knows it is running a DAW.
//!
//! So a tab is an entity, and a host declares one by spawning it — the same
//! shape `shellie-input` uses for commands and `shellie-dock` for panes.

use bevy::prelude::*;

/// Marker: this entity declares a settings tab.
///
/// Inserting it attaches [`TabLabel`] and [`TabOrder`] at their defaults
/// through Bevy's required-components chain. Override at minimum [`TabId`],
/// which has no useful default — the empty string names nothing.
#[derive(Component, Debug, Clone, Copy, Default)]
#[require(TabId, TabLabel, TabOrder)]
pub struct SettingsTab;

/// Stable identity for a tab.
///
/// Owned `String` rather than `&'static str`, matching `PaneId` and
/// `PageId`: an extension can mint one at runtime.
///
/// This is also what a session save round-trips, which is why an
/// unrecognised id is *ignorable* — see [`ActiveTab::resolve`].
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabId(pub String);

impl TabId {
    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for TabId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// The name shown in the sidebar.
#[derive(Component, Debug, Clone, Default)]
pub struct TabLabel(pub String);

impl<T: Into<String>> From<T> for TabLabel {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// Sidebar sort key; lower goes first. Ties break on [`TabId`].
///
/// A number rather than declaration order, because tabs are declared by
/// whichever crates happen to be present and system order is not a contract
/// a host should have to reason about.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TabOrder(pub i32);

/// Optional icon for the sidebar entry.
///
/// Outside [`SettingsTab`]'s require chain deliberately: not every tab has
/// one, and there is no useful default `Handle<Image>` to invent for those
/// that do not.
#[derive(Component, Debug, Clone)]
pub struct TabIcon(pub Handle<Image>);

/// Marker on the node a tab's content parents into.
///
/// The crate spawns one of these per declared tab and shows exactly the
/// active one. **A host finds its body by [`TabId`] and parents its own
/// content in** — the same inversion `shellie-dock` uses for panes, and the
/// reason this crate can host a tab whose content lives in a crate it has
/// never heard of.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TabBody;

/// Which tab is showing. Lives on the dialog root.
///
/// On the dialog rather than in a resource, so two settings dialogs — a
/// main one and a per-project one, say — do not share a selection. The same
/// reasoning that put `ActivePage` on the pane rather than in a global.
///
/// `None` shows no tab at all, which is the honest state before a host has
/// chosen or when no tabs are declared yet.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveTab(pub Option<TabId>);

impl ActiveTab {
    /// Start with `id` showing.
    pub fn at(id: impl Into<TabId>) -> Self {
        Self(Some(id.into()))
    }

    /// Show nothing.
    pub fn none() -> Self {
        Self(None)
    }

    /// Whether `id` is the active tab.
    pub fn is(&self, id: &str) -> bool {
        self.0.as_ref().is_some_and(|a| a.as_str() == id)
    }

    /// Switch to `id`.
    pub fn set(&mut self, id: impl Into<TabId>) {
        self.0 = Some(id.into());
    }

    /// The active id, if any.
    pub fn id(&self) -> Option<&str> {
        self.0.as_ref().map(TabId::as_str)
    }

    /// The active id if `available` contains it, else the first of
    /// `available`.
    ///
    /// The tolerance a restored session needs: a save naming a tab this
    /// build no longer ships must not leave the dialog blank. Same doctrine
    /// as an unknown `PaneId` or an unknown saved keybind — an id nothing
    /// claims is ignorable, not an error.
    ///
    /// `available` is expected in sidebar order, so the fallback is the
    /// first tab rather than an arbitrary one.
    pub fn resolve<'a>(&self, available: &[&'a str]) -> Option<&'a str> {
        if let Some(active) = self.id()
            && let Some(found) = available.iter().find(|id| **id == active)
        {
            return Some(found);
        }
        available.first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_gets_its_metadata_peers() {
        let mut world = World::new();
        let tab = world.spawn(SettingsTab).id();

        assert!(world.get::<TabId>(tab).is_some());
        assert!(world.get::<TabLabel>(tab).is_some());
        assert!(world.get::<TabOrder>(tab).is_some());
        assert!(
            world.get::<TabIcon>(tab).is_none(),
            "an icon has no sensible default and must stay opt-in"
        );
    }

    #[test]
    fn active_tab_matches_by_id() {
        let active = ActiveTab::at("audio");
        assert!(active.is("audio"));
        assert!(!active.is("general"));
        assert!(!ActiveTab::none().is("audio"));
        assert_eq!(active.id(), Some("audio"));
    }

    #[test]
    fn resolve_falls_back_to_the_first_tab() {
        // A session naming a tab this build dropped must not blank the
        // dialog.
        let saved = ActiveTab::at("retired");
        assert_eq!(saved.resolve(&["general", "audio"]), Some("general"));
    }

    #[test]
    fn resolve_keeps_a_tab_that_still_exists() {
        let saved = ActiveTab::at("audio");
        assert_eq!(saved.resolve(&["general", "audio"]), Some("audio"));
    }

    #[test]
    fn resolve_of_nothing_is_nothing() {
        assert_eq!(ActiveTab::at("anything").resolve(&[]), None);
        assert_eq!(ActiveTab::none().resolve(&[]), None);
    }
}
