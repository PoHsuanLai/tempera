//! Command palette — searchable list with keyboard nav.
//!
//! Mirrors shadcn's `<Command>` tree (`Command` → `CommandInput` +
//! `CommandList(CommandGroup(CommandItem...))`), built spec-driven:
//! pass a `Vec<CommandSection>` to [`spawn_command`] and tempera
//! materialises the tree. The text input field is a tempera
//! `TextInputNode` under the hood so all keystrokes flow naturally.
//!
//! ## Wiring
//!
//! ```ignore
//! use tempera::prelude::*;
//!
//! let palette = spawn_command(
//!     &mut commands,
//!     &command_style,
//!     "Search commands…",
//!     vec![
//!         CommandSection::new("Suggestions")
//!             .item(CommandItemSpec::new("new").label("New File"))
//!             .item(CommandItemSpec::new("open").label("Open…")),
//!         CommandSection::new("Settings")
//!             .item(CommandItemSpec::new("prefs").label("Preferences")),
//!     ],
//! );
//! commands.entity(palette).observe(|on: On<CommandActivated>| {
//!     info!("activated: {}", on.id);
//! });
//! ```
//!
//! ## Behavior
//!
//! - Typing in the input filters items by case-insensitive substring.
//! - `ArrowDown` / `ArrowUp` move selection.
//! - `Enter` activates the selected item.
//! - Click also activates.
//! - Groups with no matching items are hidden.
//! - When no items match, the `CommandEmpty` placeholder is shown.

use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::prelude::*;

use crate::text_input::TextInputStylePlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{
    Command, CommandActivated, CommandEmpty, CommandGroup, CommandGroupHeading, CommandInputRow,
    CommandItem, CommandItemSpec, CommandList, CommandSection, CommandSelection,
};
pub use spawn::{spawn_command, spawn_command_with_icon, CommandStyle};

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<InputDispatchPlugin>() {
            app.add_plugins(InputDispatchPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<TextInputStylePlugin>() {
            app.add_plugins(TextInputStylePlugin);
        }
        app.add_observer(systems::on_item_click);
        app.add_observer(systems::on_keyboard);
        app.add_systems(
            Update,
            (
                systems::seed_focus_on_open,
                systems::refilter,
                systems::repaint_items,
            ),
        );
    }
}
