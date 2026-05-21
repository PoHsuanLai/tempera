//! Right-click context menu — bevy_ui-based.
//!
//! Built on top of [`bevy::ui_widgets::menu`] (headless behavior:
//! focus, keyboard nav, click activation, focus-loss dismissal) and
//! [`bevy::ui_widgets::popover`] (declarative positioning for
//! submenus). Tempera adds:
//!
//! 1. **Open-at-screen-position** — Bevy's `MenuPopup` is normally
//!    opened by a `MenuButton` and anchored to it. For right-click
//!    menus we instead spawn the popup with absolute positioning
//!    computed from the cursor.
//! 2. **String-id activation** — Bevy fires an [`Activate { entity }`]
//!    trigger; we translate that into a [`MenuItemActivated { id }`]
//!    message keyed by the string the caller supplied via
//!    [`MenuItemSpec::new`].
//! 3. **Styled spawn helpers** — a `MenuRequest` of `MenuItemSpec`
//!    becomes a styled entity tree using the [`crate::theme::Theme`]
//!    resource.
//!
//! # Opening a menu
//!
//! ```ignore
//! use tempera::context_menu::{MenuItemSpec, OpenContextMenu, MenuRequest};
//!
//! fn on_right_click(mut writer: MessageWriter<OpenContextMenu>) {
//!     writer.write(OpenContextMenu(MenuRequest {
//!         anchor: Vec2::new(120.0, 80.0),
//!         items: vec![
//!             MenuItemSpec::new("rename").label("Rename"),
//!             MenuItemSpec::new("delete").label("Delete").destructive(),
//!         ],
//!     }));
//! }
//! ```
//!
//! # Receiving a click
//!
//! ```ignore
//! fn on_activate(mut reader: MessageReader<MenuItemActivated>) {
//!     for ev in reader.read() {
//!         match ev.id.as_str() {
//!             "rename" => { /* ... */ }
//!             "delete" => { /* ... */ }
//!             _ => {}
//!         }
//!     }
//! }
//! ```

use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui_widgets::MenuPlugin;
use bevy::ui_widgets::popover::PopoverPlugin;

use crate::theme::ThemePlugin;

mod components;
mod request;
mod systems;

pub use components::{MenuRootMarker, TemperaMenuItem};
pub use request::{MenuItemSpec, MenuRequest};

/// Message: open a context menu at a window-space position. Any
/// existing tempera menu is despawned first.
#[derive(Message, Debug, Clone)]
pub struct OpenContextMenu(pub MenuRequest);

/// Message: the user clicked or keyboard-activated an item. Carries
/// the string id supplied via [`MenuItemSpec::new`].
#[derive(Message, Debug, Clone)]
pub struct MenuItemActivated {
    pub id: String,
}

pub struct ContextMenuPlugin;

impl Plugin for ContextMenuPlugin {
    fn build(&self, app: &mut App) {
        // Idempotent — `is_plugin_added` keeps these registrations
        // safe if the consumer has already added the plugins (e.g.
        // because they use other bevy_ui_widgets widgets or
        // `TemperaPlugin`).
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<InputDispatchPlugin>() {
            app.add_plugins(InputDispatchPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<MenuPlugin>() {
            app.add_plugins(MenuPlugin);
        }
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }

        app.add_message::<OpenContextMenu>()
            .add_message::<MenuItemActivated>()
            .add_observer(systems::on_activate)
            .add_observer(systems::on_close_all)
            .add_systems(
                Update,
                (
                    systems::open_requested_menus,
                    systems::seed_focus_on_open,
                    systems::paint_item_highlight,
                    systems::dismiss_on_outside_right_click,
                )
                    .chain(),
            );
    }
}
