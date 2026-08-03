//! Shared behavior layer for [`crate::checkbox`] and [`crate::switch`].
//!
//! Both widgets sit on top of [`bevy::ui_widgets::Checkbox`]. This
//! plugin installs Bevy's `CheckboxPlugin` (event observers) plus
//! [`bevy::ui_widgets::checkbox_self_update`] (auto-toggles the
//! `Checked` marker on `ValueChange<bool>`). Splitting this out keeps
//! the per-widget plugins free of duplicate registrations.

use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui_widgets::{CheckboxPlugin as BevyCheckboxPlugin, checkbox_self_update};

pub struct CheckboxBehaviorPlugin;

impl Plugin for CheckboxBehaviorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<InputDispatchPlugin>() {
            app.add_plugins(InputDispatchPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<BevyCheckboxPlugin>() {
            app.add_plugins(BevyCheckboxPlugin);
        }
        app.add_observer(checkbox_self_update);
    }
}
