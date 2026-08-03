//! Tempera paints the surround (border + bg); editing behavior lives in
//! `bevy_ui_text_input`. The repaint system tints the border on focus
//! and hover.

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use super::components::TextInput;
use crate::theme::ColorPalette;

/// Paint every input's border.
///
/// One unfiltered pass, where there used to be a `dirty` query plus an
/// `all` query plus a `handled` vec to keep them from double-painting.
/// That structure existed because focus change is not a per-entity fact
/// and could not be a query filter — the same reason a palette swap could
/// not be. Both now live in the run condition, so there is one loop, and
/// [`paint`] compares before writing.
pub(crate) fn repaint_text_input(
    palette: Res<ColorPalette>,
    focus: Res<InputFocus>,
    inputs: Query<(Entity, &Interaction, Has<InteractionDisabled>), With<TextInput>>,
    mut borders: Query<&mut BorderColor, With<TextInput>>,
) {
    let focused = focus.get();
    for (entity, interaction, disabled) in &inputs {
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
    let want = BorderColor::all(with_alpha(edge, alpha));
    if *border != want {
        *border = want;
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}
