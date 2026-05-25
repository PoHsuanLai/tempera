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

/// Display precision (decimal digits). When absent, Rust's default
/// float formatting is used.
#[derive(Component, Clone, Copy, Debug)]
pub struct NumberFieldPrecision(pub usize);

/// Tag on the stepper Buttons.
#[derive(Component, Clone, Copy, Debug)]
pub enum NumberFieldKind {
    Increment,
    Decrement,
}

/// Visual config for a NumberField. Controls the dimensions, colors,
/// and rounding of the `[−] [value] [+]` strip. Callers pass the raw
/// values they want — no semantic size variants.
#[derive(Clone, Copy, Debug)]
pub struct NumberFieldConfig {
    pub input_width: f32,
    pub input_bg: Color,
    pub height: f32,
    pub gap: f32,
    pub bg: Color,
    pub bg_hover: Color,
    pub border_radius: f32,
    pub stepper_width: f32,
}

impl Default for NumberFieldConfig {
    fn default() -> Self {
        Self {
            input_width: 80.0,
            input_bg: Color::NONE,
            height: 28.0,
            gap: 2.0,
            bg: Color::NONE,
            bg_hover: Color::NONE,
            border_radius: 4.0,
            stepper_width: 28.0,
        }
    }
}
