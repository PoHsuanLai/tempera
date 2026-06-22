//! Components tempera adds on top of `bevy::ui_widgets::menu`.
//!
//! Bevy's [`MenuPopup`](bevy::ui_widgets::menu::MenuPopup) handles
//! focus, keyboard nav, dismissal, and modal trapping. Tempera adds:
//!
//! - [`TemperaMenuItem`] — carries the caller-supplied string `id` so
//!   we can fire [`super::MenuItemActivated`] with a stable handle.
//! - [`MenuRootMarker`] — distinguishes our context-menu root from
//!   other `MenuPopup`s (e.g. one opened by a `MenuButton` chain).

use bevy::prelude::*;

/// Marker for the *root* tempera context menu (the one opened by
/// `OpenContextMenu`). Submenus do not carry this.
#[derive(Component)]
pub struct MenuRootMarker {
    /// Frame the menu opened on. Used to ignore the pointer-up that
    /// follows the right-click which triggered it.
    pub opened_at_frame: u32,
}

/// Carries the caller-supplied string id and optional definition
/// entity for a menu row. Sits next to Bevy's
/// [`bevy::ui_widgets::menu::MenuItem`] marker; the latter drives
/// focus/keyboard/click behavior, this carries our identity so
/// activation can be routed back to the caller.
#[derive(Component)]
pub struct TemperaMenuItem {
    pub id: String,
    /// Mirrors [`super::request::MenuItemSpec::origin`] — the
    /// caller-side definition entity. Echoed back on activation.
    pub origin: Option<Entity>,
}
