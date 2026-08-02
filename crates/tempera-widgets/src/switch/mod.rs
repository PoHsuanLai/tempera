//! Switch — toggle pill with sliding thumb.
//!
//! Shares headless behavior with [`crate::checkbox`]: built on
//! [`bevy::ui_widgets::Checkbox`] with the accessibility role
//! overridden to `Switch`. Visually distinct — a pill-shaped track
//! with a circular thumb that springs between left and right on
//! toggle (damped-spring animation via [`crate::anim::Spring`]).
//!
//! ## Composition
//!
//! - [`Checkbox`] (re-exported) — behavior marker
//! - [`Switch`] — tempera marker so paint systems target switches, not
//!   checkboxes
//! - [`SwitchThumb`] — child marker; paint system updates its `left`
//! - [`SwitchSize`] — sizing preset
//! - [`Checked`] — checked state
//! - [`InteractionDisabled`] — disabled state

use bevy::prelude::*;

use crate::checkbox_behavior::CheckboxBehaviorPlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use bevy::ui::Checked;
pub use bevy::ui_widgets::ValueChange;
pub use components::{Switch, SwitchSize, SwitchThumb};
pub use spawn::{spawn_switch, SwitchStyle};

pub struct SwitchStylePlugin;

impl Plugin for SwitchStylePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<CheckboxBehaviorPlugin>() {
            app.add_plugins(CheckboxBehaviorPlugin);
        }
        app.add_systems(
            Update,
            (
                systems::retarget_switch,
                systems::repaint_switch_track,
                systems::drive_switch,
            )
                .chain(),
        );
    }
}
