//! ToggleGroup — segmented control.
//!
//! Two variants, sharing visuals:
//!
//! - **Single** wraps [`bevy::ui_widgets::RadioGroup`] +
//!   [`bevy::ui_widgets::RadioButton`]. Exclusive selection. Group
//!   emits `ValueChange<Entity>` carrying the newly-selected button.
//! - **Multiple** is a row of independent [`bevy::ui_widgets::Checkbox`]es
//!   styled as toggles. Each item emits `ValueChange<bool>`.
//!
//! Tempera installs an auto-update observer for single-mode groups so
//! `Checked` follows the selection without the app having to wire it.
//!
//! ## Composition
//!
//! - [`ToggleGroup`] — root marker (carries [`ToggleGroupKind`])
//! - [`ToggleItem`] — child marker, styles a row item visually
//! - [`Checked`] — selected state (toggled by Bevy's behavior layer +
//!   our auto-update observer)

use bevy::prelude::*;

use crate::checkbox_behavior::CheckboxBehaviorPlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use bevy::ui::Checked;
pub use bevy::ui_widgets::{RadioButton, RadioGroup, ValueChange};
pub use components::{ToggleGroup, ToggleGroupKind, ToggleItem};
pub use spawn::{spawn_toggle_group, ToggleGroupItem, ToggleGroupStyle};

pub struct ToggleGroupStylePlugin;

impl Plugin for ToggleGroupStylePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        // Bevy 0.18's RadioGroupPlugin handles single-mode behavior;
        // CheckboxBehaviorPlugin handles multi-mode.
        if !app.is_plugin_added::<bevy::ui_widgets::RadioGroupPlugin>() {
            app.add_plugins(bevy::ui_widgets::RadioGroupPlugin);
        }
        if !app.is_plugin_added::<CheckboxBehaviorPlugin>() {
            app.add_plugins(CheckboxBehaviorPlugin);
        }
        app.add_observer(systems::radio_group_self_update);
        app.add_systems(Update, systems::repaint_toggle_items);
    }
}
