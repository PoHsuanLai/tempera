//! Dropdown menu — a button that opens a tempera menu popup anchored
//! to itself.
//!
//! Reuses the styled menu spawn helpers from [`crate::context_menu`]
//! (same `MenuItemSpec`, same `MenuItemActivated` event). The only
//! addition over right-click context menus is the **trigger** — a
//! [`Button`] that, on activation, computes the trigger's window-space
//! rect and opens the menu beneath it.
//!
//! ## Spawning
//!
//! ```ignore
//! use tempera::dropdown_menu::{spawn_dropdown, DropdownItems};
//!
//! let items = vec![
//!     MenuItemSpec::new("save").label("Save").shortcut("⌘S"),
//!     MenuItemSpec::new("save_as").label("Save As…"),
//! ];
//! let id = spawn_dropdown(&mut commands, &style, "File", items);
//! ```
//!
//! Receive activations through the same channel as context menus:
//!
//! ```ignore
//! fn on_activate(mut reader: MessageReader<MenuItemActivated>) {
//!     for ev in reader.read() { ... }
//! }
//! ```

use bevy::prelude::*;

use crate::context_menu::ContextMenuPlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::DropdownTrigger;
pub use spawn::{DropdownStyle, spawn_dropdown};

pub struct DropdownMenuPlugin;

impl Plugin for DropdownMenuPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        // Pull the full context-menu stack — that's where the popup
        // spawn / activation / dismissal lives.
        if !app.is_plugin_added::<ContextMenuPlugin>() {
            app.add_plugins(ContextMenuPlugin);
        }
        app.add_observer(systems::open_on_trigger_activate);
    }
}
