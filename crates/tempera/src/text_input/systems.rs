//! Tempera paints the surround (border + bg); editing behavior lives in
//! `bevy_ui_text_input`. The repaint system tints the border on focus
//! and hover.

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use super::components::TextInput;
use crate::theme::ColorPalette;

pub(crate) fn repaint_text_input(
    palette: Res<ColorPalette>,
    focus: Res<InputFocus>,
    dirty: Query<
        (Entity, &Interaction, Has<InteractionDisabled>),
        (
            With<TextInput>,
            Or<(Changed<Interaction>, Changed<InteractionDisabled>, Added<TextInput>)>,
        ),
    >,
    all: Query<(Entity, &Interaction, Has<InteractionDisabled>), With<TextInput>>,
    mut borders: Query<&mut BorderColor, With<TextInput>>,
) {
    let focused = focus.0;

    let mut handled: Vec<Entity> = Vec::new();
    for (entity, interaction, disabled) in &dirty {
        handled.push(entity);
        paint(&palette, entity, interaction, disabled, focused == Some(entity), &mut borders);
    }

    // On focus changes, repaint inputs whose focused state may have
    // flipped (so the previously-focused one drops back to its
    // hover/idle border).
    if focus.is_changed() {
        for (entity, interaction, disabled) in &all {
            if handled.contains(&entity) {
                continue;
            }
            paint(
                &palette,
                entity,
                interaction,
                disabled,
                focused == Some(entity),
                &mut borders,
            );
        }
    }
}

fn paint(
    palette: &ColorPalette,
    entity: Entity,
    interaction: &Interaction,
    disabled: bool,
    focused: bool,
    borders: &mut Query<&mut BorderColor, With<TextInput>>,
) {
    let alpha = if disabled { 0.5 } else { 1.0 };
    let hovered = !disabled && matches!(interaction, Interaction::Hovered | Interaction::Pressed);
    let Ok(mut border) = borders.get_mut(entity) else {
        return;
    };
    let edge = if focused {
        palette.ring
    } else if hovered {
        ColorPalette::hover_lift(palette.input, 0.12)
    } else {
        palette.input
    };
    *border = BorderColor::all(with_alpha(edge, alpha));
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}
