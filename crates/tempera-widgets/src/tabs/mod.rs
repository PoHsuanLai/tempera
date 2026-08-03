//! Tabs — horizontal segmented switcher with an animated indicator.
//!
//! Bevy 0.18's `bevy_ui_widgets` doesn't ship a Tabs primitive, so
//! tempera owns the behavior here. A tabs row is:
//!
//! - **Root** ([`Tabs`] + [`TabsActive`]) — a Node carrying the active
//!   index and emitting [`TabsChanged`] when it flips.
//! - **Triggers** ([`TabTrigger`]) — children with a click handler.
//! - **Indicator** ([`TabIndicator`]) — child Node lerped toward the
//!   active trigger's position by the paint system.
//!
//! ## Spawning
//!
//! ```ignore
//! let id = spawn_tabs(&mut commands, &style, vec!["Files", "Search", "Git"], 0);
//! commands.entity(id).observe(|on: On<TabsChanged>| info!("tab -> {}", on.active));
//! ```

use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui_widgets::ButtonPlugin as BevyButtonPlugin;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{TabIndicator, TabTrigger, Tabs, TabsActive};
pub use spawn::{TabsStyle, spawn_tabs};
pub use systems::TabsChanged;

pub struct TabsPlugin;

impl Plugin for TabsPlugin {
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
        if !app.is_plugin_added::<BevyButtonPlugin>() {
            app.add_plugins(BevyButtonPlugin);
        }
        // TabsChanged is an EntityEvent triggered via `commands.trigger`;
        // no add_message registration needed.
        app.add_observer(systems::trigger_on_activate);
        app.add_systems(
            Update,
            (
                systems::repaint_triggers
                    .run_if(crate::theme::repaint_needed_on::<TabTrigger, Changed<TabsActive>>),
                systems::move_indicator,
            ),
        );
    }
}
