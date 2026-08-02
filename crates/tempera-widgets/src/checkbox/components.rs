use bevy::prelude::*;

/// Tempera marker on the standalone checkbox root. Distinguishes a
/// tempera-styled `spawn_checkbox` from other entities that also carry
/// the headless [`bevy::ui_widgets::Checkbox`] behavior (e.g.
/// toggle-group items in Multiple mode, the switch widget). Paint
/// systems filter on this marker so those entities don't get the
/// checkbox visual treatment.
#[derive(Component, Default, Debug)]
pub struct TemperaCheckbox;

/// Marker on the check-glyph child Node. The paint system toggles its
/// `Display` between `None` (unchecked) and `Flex` (checked).
#[derive(Component, Default, Debug)]
pub struct CheckGlyph;
