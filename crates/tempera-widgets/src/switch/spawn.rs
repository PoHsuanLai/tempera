use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui_widgets::Checkbox;

use super::components::{Switch, SwitchSize, SwitchThumb};
use crate::anim::Spring;
use crate::theme::ColorPalette;

#[derive(SystemParam)]
pub struct SwitchStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
}

/// Spawn a styled switch at the default size. Returns the root entity.
/// Observe `ValueChange<bool>` on it to react to flips.
pub fn spawn_switch(commands: &mut Commands, style: &SwitchStyle, checked: bool) -> Entity {
    spawn_switch_sized(commands, style, checked, SwitchSize::default())
}

/// Spawn a styled switch at an explicit [`SwitchSize`].
///
/// # Why this exists separately
///
/// [`spawn_switch`] used to hardcode `SwitchSize::default()` and drop the
/// component's other two variants on the floor — the type was fully
/// specified, the spawn path just never read it. A caller wanting a small
/// switch had to insert `SwitchSize::Sm` and then resize the thumb child by
/// hand afterwards, because the track and thumb are laid out here and never
/// recomputed. dawai carried a whole system to do exactly that, complete
/// with a marker component so it would not run twice.
///
/// The pairing mirrors [`crate::button::spawn_button`] /
/// `spawn_button_sized`.
pub fn spawn_switch_sized(
    commands: &mut Commands,
    style: &SwitchStyle,
    checked: bool,
    size: SwitchSize,
) -> Entity {
    let initial_t = if checked { 1.0 } else { 0.0 };
    let mut root = commands.spawn((
        Switch,
        Checkbox,
        size,
        // Spring drives the thumb position. Seeded at the initial state
        // so the switch doesn't animate in from 0 on spawn. Its feel
        // (`SPRING_K` / `SPRING_DAMPING`) is applied by `drive_switch`,
        // which is the only thing that steps it.
        Spring::new(initial_t),
        Node {
            width: Val::Px(size.track_width()),
            height: Val::Px(size.track_height()),
            border_radius: BorderRadius::all(Val::Px(size.track_height() * 0.5)),
            position_type: PositionType::Relative,
            ..default()
        },
        BackgroundColor(if checked {
            style.palette.primary
        } else {
            style.palette.muted
        }),
        Interaction::default(),
        crate::cursor::HoverCursor::default(),
        Name::new("tempera::switch"),
    ));
    if checked {
        root.insert(bevy::ui::Checked);
    }
    let id = root.id();

    commands.spawn((
        SwitchThumb,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(SwitchSize::INSET),
            left: Val::Px(SwitchSize::INSET + if checked { size.thumb_travel() } else { 0.0 }),
            width: Val::Px(size.thumb_diameter()),
            height: Val::Px(size.thumb_diameter()),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        // Initial thumb color — repaint flips this between
        // `primary_foreground` (ON, contrasts the white track) and
        // `foreground` (OFF, contrasts the muted track).
        BackgroundColor(if checked {
            style.palette.primary_foreground
        } else {
            style.palette.foreground
        }),
        bevy::picking::Pickable::IGNORE,
        ChildOf(id),
    ));

    id
}
