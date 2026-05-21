use bevy::prelude::*;

#[derive(Component, Default, Debug)]
pub struct NumberField;

#[derive(Component, Clone, Copy, Debug)]
pub struct NumberFieldValue(pub f32);

impl Default for NumberFieldValue {
    fn default() -> Self {
        Self(0.0)
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct NumberFieldRange {
    pub min: f32,
    pub max: f32,
}

impl Default for NumberFieldRange {
    fn default() -> Self {
        Self {
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct NumberFieldStep(pub f32);

impl Default for NumberFieldStep {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Tag on the stepper Buttons.
#[derive(Component, Clone, Copy, Debug)]
pub enum NumberFieldKind {
    Increment,
    Decrement,
}
