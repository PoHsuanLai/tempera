//! Builder data — what the caller hands us when opening a menu.

use bevy::prelude::*;

use crate::kbd::KbdChord;

/// One menu entry. `id` is opaque to the menu — it's echoed back via
/// [`super::MenuItemActivated`] so the caller can route the click.
#[derive(Debug, Clone)]
pub struct MenuItemSpec {
    pub id: String,
    pub label: String,
    pub shortcut: Option<KbdChord>,
    pub destructive: bool,
    pub separator_before: bool,
    pub enabled: bool,
}

impl MenuItemSpec {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: String::new(),
            shortcut: None,
            destructive: false,
            separator_before: false,
            enabled: true,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    #[must_use]
    pub fn shortcut(mut self, chord: impl Into<KbdChord>) -> Self {
        self.shortcut = Some(chord.into());
        self
    }

    #[must_use]
    pub const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    #[must_use]
    pub const fn separator_before(mut self) -> Self {
        self.separator_before = true;
        self
    }

    #[must_use]
    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Complete menu open request: where to anchor + what to show.
#[derive(Debug, Clone)]
pub struct MenuRequest {
    /// Window-space position (logical pixels, top-left origin) where
    /// the menu's top-left corner should land. The plugin will nudge
    /// the menu back inside the window if it would clip.
    pub anchor: Vec2,
    pub items: Vec<MenuItemSpec>,
}
