//! Builder data — what the caller hands us when opening a menu.

use bevy::prelude::*;

use crate::kbd::KbdChord;

/// One menu entry.
///
/// `id` is a string handle echoed back via [`super::MenuItemActivated`]
/// — useful for callers that route by name. `origin` carries an
/// optional ECS entity that owns the item's *definition* (e.g. a
/// `InContextMenu` registry entity, or any caller-side identity); when
/// set, the same entity is reported on activation so callers can
/// attach behavior directly to it and skip the string lookup.
#[derive(Debug, Clone)]
pub struct MenuItemSpec {
    pub id: String,
    pub label: String,
    pub shortcut: Option<KbdChord>,
    pub destructive: bool,
    pub separator_before: bool,
    pub enabled: bool,
    /// Optional caller-side "definition" entity. Echoed back via
    /// [`super::MenuItemActivated::entity`]. Lets the caller attach
    /// activation logic as components on the entity instead of routing
    /// through a string-keyed registry.
    pub origin: Option<Entity>,
    /// Child items. When non-empty, this item renders a "▸" arrow and
    /// opens a submenu on hover.
    pub children: Vec<MenuItemSpec>,
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
            origin: None,
            children: Vec::new(),
        }
    }

    /// Attach a caller-side definition entity. When the user activates
    /// the item, [`super::MenuItemActivated::entity`] will be `Some(this)`.
    #[must_use]
    pub fn origin(mut self, entity: Entity) -> Self {
        self.origin = Some(entity);
        self
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

    #[must_use]
    pub fn children(mut self, items: Vec<MenuItemSpec>) -> Self {
        self.children = items;
        self
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
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
