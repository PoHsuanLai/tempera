//! NumberField — numeric input with +/- steppers.
//!
//! Composes a [`crate::text_input::TextInput`] flanked by two
//! [`crate::button::Button`] children. The text input filters
//! non-numeric characters on commit (Enter or blur), and the
//! steppers nudge the value by [`NumberFieldStep`].
//!
//! ## Composition
//!
//! - [`NumberField`] — root marker
//! - [`NumberFieldValue`] — current f32 value
//! - [`NumberFieldRange`] — min/max clamps
//! - [`NumberFieldStep`] — increment used by the steppers
//! - [`NumberFieldKind::Increment`] / `Decrement` — child marker on
//!   the two stepper buttons

use bevy::prelude::*;

use crate::button::ButtonStylePlugin;
use crate::text_input::TextInputStylePlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{
    NumberField, NumberFieldConfig, NumberFieldKind, NumberFieldPrecision, NumberFieldRange,
    NumberFieldStep, NumberFieldValue,
};
pub use spawn::{NumberFieldStyle, spawn_number_field, spawn_number_field_configured};
pub use systems::NumberFieldValueChange as ValueChange;

pub struct NumberFieldPlugin;

impl Plugin for NumberFieldPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<ButtonStylePlugin>() {
            app.add_plugins(ButtonStylePlugin);
        }
        if !app.is_plugin_added::<TextInputStylePlugin>() {
            app.add_plugins(TextInputStylePlugin);
        }
        app.add_observer(systems::stepper_on_activate);
        app.add_systems(
            Update,
            (
                systems::sync_buffer_from_value,
                systems::sync_value_from_buffer,
            ),
        );
    }
}
