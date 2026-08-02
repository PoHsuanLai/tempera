use bevy::prelude::*;

/// Tempera marker on a styled text-input root. The behavior layer is
/// [`bevy_ui_text_input::TextInputNode`] (added as a child of the
/// styled surround) — this marker lets the repaint system find the
/// surround entity quickly.
#[derive(Component, Default, Debug)]
pub struct TextInput;
