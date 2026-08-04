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
//!
//! # Two ways in, and when each applies
//!
//! Everything above is the **spec** path: you build the `Vec<MenuItemSpec>`
//! and open it. That is the right shape when one place knows the whole
//! menu — a dropdown with four fixed entries.
//!
//! It stops working the moment the entries arrive from elsewhere. A menu
//! that a plugin, an extension, or a feature module contributes to has no
//! single place to write that `Vec`. For that there is
//! [`registry`](self::registry) — items are entities tagged with a surface,
//! and [`open_surface`] collects them. See its docs; the two paths meet at
//! `MenuItemSpec`, so the renderer below is shared.

use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::{InputDispatchPlugin, InputFocusPlugin};
use bevy::prelude::*;
use bevy::ui_widgets::MenuPlugin;
use bevy::ui_widgets::popover::PopoverPlugin;

use crate::theme::ThemePlugin;

mod components;
mod registry;
mod request;
mod systems;

pub use components::{HasSubMenu, MenuRootMarker, SubMenuChild, SubMenuOf, TemperaMenuItem};
pub use registry::{
    AppMenuExt, Destructive, MenuClosed, MenuDisabled, MenuItemMarker, MenuLabel, MenuOrder,
    MenuShortcut, MenuShortcutFor, MenuSurface, SeparatorBefore, VisibleWhen, child_item,
    collect_surface, menu_item, open_surface,
};
pub use request::{MenuItemSpec, MenuRequest};

/// Message: open a context menu at a window-space position. Any
/// existing tempera menu is despawned first.
#[derive(Message, Debug, Clone)]
pub struct OpenContextMenu(pub MenuRequest);

/// Message: the user clicked or keyboard-activated an item.
///
/// Carries both the string `id` supplied via [`MenuItemSpec::new`]
/// and the caller-side definition `entity` from
/// [`MenuItemSpec::origin`] (when set). Routers can match on
/// whichever is more convenient; `entity` is preferred when the
/// caller wants to attach activation behavior as components.
#[derive(Message, Debug, Clone)]
pub struct MenuItemActivated {
    pub id: String,
    pub entity: Option<Entity>,
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
        // `InputDispatchPlugin` adds `dispatch_focused_input`, which reads an
        // `InputFocus` resource it does not insert — `InputFocusPlugin` owns
        // that. Adding the dispatcher alone leaves systems scheduled against a
        // missing resource, and a missing resource fails system-parameter
        // validation outright rather than reading empty, so the app panics on
        // its first frame.
        if !app.is_plugin_added::<InputFocusPlugin>() {
            app.add_plugins(InputFocusPlugin);
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

        systems::observe_submenu_hover(app);

        // `paint_item_highlight` and `MenuStyle` read `MenuTokens`. It has a
        // `Default`, but nothing was calling it — so every system below was
        // scheduled against a resource no code path inserted, and the plugin
        // could not be added to an app without panicking.
        // The tokens are derived from the palette, not constants: three of
        // them were hardcoded white-alpha lifts that read as white-on-white
        // in a light theme. `sync_menu_tokens` owns keeping them current.
        app.init_resource::<crate::menu_tokens::MenuTokens>()
            .add_systems(Update, crate::menu_tokens::sync_menu_tokens)
            .add_message::<OpenContextMenu>()
            .add_message::<MenuItemActivated>()
            .add_message::<MenuClosed>()
            .add_observer(systems::on_activate)
            .add_observer(systems::on_close_all)
            .add_observer(registry::report_menu_closed)
            .add_systems(
                Update,
                (
                    systems::open_requested_menus,
                    systems::seed_focus_on_open,
                    systems::tick_submenu_close_timers,
                    systems::manage_submenus,
                    systems::paint_item_highlight,
                    systems::dismiss_on_outside_right_click,
                )
                    .chain(),
            );
    }
}
