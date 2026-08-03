//! Select — a dropdown value picker.
//!
//! Shadcn-style select: a trigger button showing the current value +
//! chevron, which opens a context-menu popup listing all options. The
//! selected item is marked with a checkmark. Picking a new option
//! updates the display text and fires a [`ValueChange`] event on the
//! select entity.
//!
//! ## Spawning
//!
//! ```ignore
//! use tempera::select::{spawn_select, SelectOption, SelectStyle};
//!
//! let options = vec![
//!     SelectOption { id: "master".into(), label: "Master".into() },
//!     SelectOption { id: "bus_a".into(), label: "Bus A".into() },
//! ];
//! let id = spawn_select(&mut commands, &style, options, "master");
//! commands.entity(id).observe(|trigger: On<ValueChange>| {
//!     let ev = trigger.event();
//!     println!("selected: {} ({})", ev.label, ev.value);
//! });
//! ```

use bevy::prelude::*;

use crate::context_menu::ContextMenuPlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{Select, SelectOption, SelectOptions, SelectValue, ValueChange};
pub use spawn::{SelectStyle, spawn_select};

pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        info!("[SelectPlugin] registering");
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<ContextMenuPlugin>() {
            app.add_plugins(ContextMenuPlugin);
        }
        app.add_observer(systems::open_on_click).add_systems(
            Update,
            (
                systems::on_menu_item_activated,
                systems::paint_select_hover.run_if(
                    crate::theme::repaint_needed::<Select>
                        .or_else(resource_changed::<crate::menu_tokens::MenuTokens>),
                ),
            ),
        );
    }
}
