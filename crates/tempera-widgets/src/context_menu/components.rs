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

/// The row renders in the destructive style.
///
/// A *declaration*, not a colour: the paint system resolves it against
/// the live palette, so nothing here names a `Color`.
///
/// It has to exist as a component because a repaint has to be able to
/// answer "what colour should this label be?" long after the spawn that
/// first answered it. `MenuItemSpec::destructive` is a spawn argument
/// and is consumed there; the disabled case was already recoverable from
/// [`InteractionDisabled`](bevy::ui_widgets::InteractionDisabled), and
/// this is the same fact for the remaining branch.
#[derive(Component)]
pub struct DestructiveRow;

/// Text inside a menu row that follows the row's own foreground.
///
/// The label and the submenu arrow resolve to *different* colours from
/// the same row state, so they cannot share a marker: an arrow is always
/// `muted_foreground`, while a label is muted only when the row is
/// disabled. Marking the label specifically is what lets one system
/// paint both without re-deriving which child is which from the
/// hierarchy.
#[derive(Component)]
pub struct MenuItemLabel;

/// Text inside a menu row that is always `muted_foreground` — the
/// submenu arrow and the inline shortcut chord.
#[derive(Component)]
pub struct MenuItemMutedText;

/// The popover surface of a menu (root or submenu).
///
/// Both carry it: a submenu is a peer popup with the same chrome, and a
/// theme change reaches whichever are open.
///
/// Named for the *chrome*, not the menu: [`super::registry::MenuSurface`]
/// already means "which menu an item belongs to", which is a different
/// idea that happens to want the same word.
#[derive(Component)]
pub struct MenuPopoverSurface;

/// A separator rule between menu rows.
#[derive(Component)]
pub struct MenuSeparator;

/// Marks a `MenuPopup` as a submenu spawned by a parent item.
/// The entity field points to the parent menu item row that owns it.
#[derive(Component)]
pub struct SubMenuOf(pub Entity);

/// Marks a menu item as having a submenu. Stores the child specs so
/// the hover system can spawn them.
#[derive(Component)]
pub struct HasSubMenu(pub Vec<super::request::MenuItemSpec>);

/// Marker on every entity inside a spawned submenu popup (the popup
/// container and all its item rows). Used to distinguish submenu
/// children from regular menu items so the close-on-hover observer
/// doesn't dismiss the submenu when the cursor enters it.
#[derive(Component)]
pub struct SubMenuChild;
