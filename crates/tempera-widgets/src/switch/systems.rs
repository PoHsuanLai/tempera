use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};

use super::components::{Switch, SwitchSize, SwitchThumb};
use crate::anim::Spring;
use crate::theme::{ColorPalette, HOVER};

/// Set the spring target whenever `Checked` flips. The actual thumb
/// position is interpolated each frame by [`drive_switch`].
///
/// We can't filter on `Changed<Checked>` alone because `Checked` is a
/// marker that gets *removed* on toggle-off, and Bevy's change
/// detection fires `Changed<T>` on insert/mutate but not on removal.
/// Running the target update on every switch each frame is cheap (a
/// scalar comparison) and avoids that edge case entirely.
pub(crate) fn retarget_switch(mut switches: Query<(Has<Checked>, &mut Spring), With<Switch>>) {
    for (checked, mut spring) in &mut switches {
        let target = if checked { 1.0 } else { 0.0 };
        if (spring.target - target).abs() > f32::EPSILON {
            spring.set_target(target);
        }
    }
}

/// Repaint switch track + thumb colors when hover / disabled / checked
/// changes. The thumb color flips between `foreground` (contrasts the
/// `muted` OFF-track) and `primary_foreground` (contrasts the `primary`
/// ON-track) so the thumb never blends into the track. Color changes
/// are snaps — only the thumb position springs.
pub(crate) fn repaint_switch_track(
    palette: Res<ColorPalette>,
    switches: Query<
        (
            Entity,
            Has<Checked>,
            &Interaction,
            Has<InteractionDisabled>,
            &Children,
        ),
        With<Switch>,
    >,
    mut track_bg: Query<&mut BackgroundColor, (With<Switch>, Without<SwitchThumb>)>,
    mut thumb_bg: Query<&mut BackgroundColor, (With<SwitchThumb>, Without<Switch>)>,
) {
    for (entity, checked, interaction, disabled, kids) in &switches {
        let alpha = if disabled { 0.5 } else { 1.0 };
        let hovered =
            !disabled && matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        let base = if checked {
            palette.primary
        } else {
            palette.muted
        };
        let fill = if hovered {
            ColorPalette::step(base, palette.background, HOVER)
        } else {
            base
        };
        let track = with_alpha(fill, alpha);
        if let Ok(mut b) = track_bg.get_mut(entity)
            && b.0 != track
        {
            *b = BackgroundColor(track);
        }
        let thumb_color = if checked {
            palette.primary_foreground
        } else {
            palette.foreground
        };
        let thumb = with_alpha(thumb_color, alpha);
        for child in kids.iter() {
            if let Ok(mut b) = thumb_bg.get_mut(child)
                && b.0 != thumb
            {
                *b = BackgroundColor(thumb);
            }
        }
    }
}

/// Drive the spring forward each frame and write the thumb position.
/// Runs every frame on any switch whose spring hasn't settled.
pub(crate) fn drive_switch(
    time: Res<Time>,
    mut switches: Query<(&mut Spring, Option<&SwitchSize>, &Children), With<Switch>>,
    mut thumbs: Query<&mut Node, With<SwitchThumb>>,
) {
    let dt = time.delta_secs();
    for (mut spring, size, kids) in &mut switches {
        if spring.settled() {
            continue;
        }
        spring.update(dt);
        let size = size.copied().unwrap_or_default();
        let t = spring.value.clamp(0.0, 1.0);
        let thumb_left = SwitchSize::INSET + t * size.thumb_travel();
        for child in kids.iter() {
            if let Ok(mut node) = thumbs.get_mut(child) {
                node.left = Val::Px(thumb_left);
            }
        }
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}
