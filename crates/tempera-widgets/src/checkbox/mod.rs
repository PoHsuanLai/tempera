//! Checkbox — small square with an animated check glyph.
//!
//! Built on top of [`bevy::ui_widgets::Checkbox`]. Headless behavior
//! (click / Space / Enter toggle, focus tracking, accessibility role)
//! is owned by Bevy; tempera adds the visuals and installs
//! [`bevy::ui_widgets::checkbox_self_update`] so the entity's
//! [`Checked`] marker tracks user input automatically.
//!
//! ## Composition
//!
//! - [`Checkbox`] — behavior marker
//! - [`Checked`] (Bevy built-in) — checked state
//! - [`CheckGlyph`] — child marker; visibility toggled with `Checked`
//! - [`InteractionDisabled`] — disabled state
//!
//! Observe `ValueChange<bool>` on the root entity to react to flips.

use bevy::prelude::*;

use crate::checkbox_behavior::CheckboxBehaviorPlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use bevy::ui::Checked;
pub use bevy::ui_widgets::{Checkbox, ValueChange};
pub use components::{CheckGlyph, TemperaCheckbox};
pub use spawn::{spawn_checkbox, CheckboxStyle};

pub struct CheckboxStylePlugin;

impl Plugin for CheckboxStylePlugin {
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
                systems::retarget_checkbox,
                systems::repaint_checkbox,
                systems::drive_checkbox,
            )
                .chain(),
        );
    }
}
