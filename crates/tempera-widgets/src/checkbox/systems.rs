use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};

use super::components::{CheckGlyph, TemperaCheckbox};
use crate::anim::Spring;
use crate::theme::{ColorPalette, HOVER};

/// Re-target the spring on `Checked` flips. Runs unfiltered every
/// frame: `Checked` is a marker that's *removed* on toggle-off, and
/// Bevy's `Changed<T>` doesn't fire on removal — so a filtered query
/// would miss the ON→OFF transition.
pub(crate) fn retarget_checkbox(
    mut boxes: Query<(Has<Checked>, &mut Spring), With<TemperaCheckbox>>,
) {
    for (checked, mut spring) in &mut boxes {
        let target = if checked { 1.0 } else { 0.0 };
        if (spring.target - target).abs() > f32::EPSILON {
            spring.set_target(target);
        }
    }
}

/// Repaint the box surface (background + border) when state flips.
/// Surface is a snap to match armas; only the checkmark itself
/// animates.
pub(crate) fn repaint_checkbox(
    palette: Res<ColorPalette>,
    boxes: Query<
        (Entity, Has<Checked>, &Interaction, Has<InteractionDisabled>),
        With<TemperaCheckbox>,
    >,
    mut bg: Query<&mut BackgroundColor, With<TemperaCheckbox>>,
    mut border: Query<&mut BorderColor, With<TemperaCheckbox>>,
) {
    for (entity, checked, interaction, disabled) in &boxes {
        let alpha = if disabled { 0.5 } else { 1.0 };
        let hovered =
            !disabled && matches!(interaction, Interaction::Hovered | Interaction::Pressed);

        let fill = if checked {
            with_alpha(palette.primary, alpha)
        } else {
            with_alpha(palette.background, alpha)
        };
        if let Ok(mut bg) = bg.get_mut(entity)
            && bg.0 != fill
        {
            *bg = BackgroundColor(fill);
        }

        let base_edge = if checked {
            palette.primary
        } else {
            palette.input
        };
        let edge = if hovered {
            ColorPalette::step(base_edge, palette.background, HOVER)
        } else {
            base_edge
        };
        let want = BorderColor::all(with_alpha(edge, alpha));
        if let Ok(mut b) = border.get_mut(entity)
            && *b != want
        {
            *b = want;
        }
    }
}

/// Drive the spring and apply scale + alpha to the glyph child.
pub(crate) fn drive_checkbox(
    time: Res<Time>,
    palette: Res<ColorPalette>,
    mut boxes: Query<(&mut Spring, &Children), With<TemperaCheckbox>>,
    mut glyphs: Query<(&mut Transform, &mut TextColor), With<CheckGlyph>>,
) {
    let dt = time.delta_secs();
    for (mut spring, kids) in &mut boxes {
        if spring.settled() {
            continue;
        }
        spring.update(dt);
        let t = spring.value.clamp(0.0, 1.0);
        let scale = t;
        let alpha = (t * 1.5).clamp(0.0, 1.0);
        for child in kids.iter() {
            if let Ok((mut transform, mut color)) = glyphs.get_mut(child) {
                transform.scale = Vec3::splat(scale);
                let mut c = palette.primary_foreground;
                c.set_alpha(alpha);
                color.0 = c;
            }
        }
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}
