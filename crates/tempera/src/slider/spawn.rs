//! Spawn helper + `SliderStyle` SystemParam bundle.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui_widgets::{Slider, SliderRange, SliderThumb, SliderValue};

use super::components::{SliderFill, SliderSize, SliderTrack};
use crate::theme::ColorPalette;

/// Theme tokens read by slider spawn / paint systems.
#[derive(SystemParam)]
pub struct SliderStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
}

/// Spawn a styled slider. Returns the root entity (which carries
/// [`Slider`], `SliderValue`, `SliderRange`, `SliderStep`). Add
/// [`SliderSize`] to override the default sizing.
///
/// The returned entity is the value owner — observe `ValueChange<f32>`
/// on it to react to drag/keyboard changes.
pub fn spawn_slider(
    commands: &mut Commands,
    style: &SliderStyle,
    range: SliderRange,
    value: SliderValue,
) -> Entity {
    let size = SliderSize::default();

    // Root row container — flex so the track centers vertically and
    // the thumb can overflow its top/bottom.
    let mut root = commands.spawn((
        Slider::default(),
        range,
        value,
        size,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(size.row_height()),
            align_items: AlignItems::Center,
            ..default()
        },
        Interaction::default(),
        crate::cursor::HoverCursor::default(),
        Name::new("tempera::slider"),
    ));
    let slider_id = root.id();

    root.with_children(|parent| {
        // Track — visual horizontal bar.
        let mut track = parent.spawn((
            SliderTrack,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(size.track_height()),
                border_radius: BorderRadius::all(Val::Px(size.track_height() * 0.5)),
                position_type: PositionType::Relative,
                ..default()
            },
            BackgroundColor(style.palette.muted),
            bevy::picking::Pickable::IGNORE,
        ));

        track.with_children(|track| {
            // Fill — stretched to thumb_position by the paint system.
            track.spawn((
                SliderFill,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    border_radius: BorderRadius::all(Val::Px(size.track_height() * 0.5)),
                    ..default()
                },
                BackgroundColor(style.palette.primary),
                bevy::picking::Pickable::IGNORE,
            ));

            // Thumb — overflows the track, positioned by paint system.
            track.spawn((
                SliderThumb,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(-(size.thumb_diameter() - size.track_height()) * 0.5),
                    width: Val::Px(size.thumb_diameter()),
                    height: Val::Px(size.thumb_diameter()),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(style.palette.background),
                BorderColor::all(style.palette.primary),
            ));
        });
    });

    slider_id
}
