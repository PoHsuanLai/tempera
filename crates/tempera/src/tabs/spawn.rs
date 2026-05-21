use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use super::components::{TabIndicator, TabTrigger, Tabs, TabsActive};
use crate::theme::{ColorPalette, FontHandle, Spacing, Typography};

const HEIGHT: f32 = 32.0;
const TRIGGER_PADDING_X: f32 = 14.0;
pub(crate) const INDICATOR_INSET: f32 = 2.0;

#[derive(SystemParam)]
pub struct TabsStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub spacing: Res<'w, Spacing>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
}

/// Spawn a tabs row. `labels` are the visible trigger names; `active`
/// is the initial selected index (clamped to `0..labels.len()`).
pub fn spawn_tabs(
    commands: &mut Commands,
    style: &TabsStyle,
    labels: Vec<String>,
    active: usize,
) -> Entity {
    let active = active.min(labels.len().saturating_sub(1));

    let root = commands
        .spawn((
            Tabs,
            TabsActive(active),
            Node {
                flex_direction: FlexDirection::Row,
                height: Val::Px(HEIGHT),
                padding: UiRect::all(Val::Px(INDICATOR_INSET)),
                border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small)),
                position_type: PositionType::Relative,
                // shadcn's tabs use `w-fit` — shrink to content rather
                // than expand to the parent's cross-axis. A
                // column-flex parent's default `align_items: Stretch`
                // would otherwise blow the track out to full width.
                align_self: AlignSelf::FlexStart,
                width: Val::Auto,
                ..default()
            },
            BackgroundColor(style.palette.muted),
            Name::new("tempera::tabs"),
        ))
        .id();

    // Indicator first so triggers paint on top of it. Matches shadcn
    // dark mode: a faint `input` fill at 30% alpha with a 1px `input`
    // border — subtle pill behind the active trigger rather than a
    // solid `background` block (which would clash with the slightly
    // brighter `muted` track introduced for toggle contrast).
    let mut indicator_fill = style.palette.input;
    indicator_fill.set_alpha(0.3);
    commands.spawn((
        TabIndicator,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(INDICATOR_INSET),
            left: Val::Px(INDICATOR_INSET),
            width: Val::Px(0.0),
            height: Val::Px(HEIGHT - INDICATOR_INSET * 2.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small - 2.0)),
            ..default()
        },
        BackgroundColor(indicator_fill),
        BorderColor::all(style.palette.input),
        bevy::picking::Pickable::IGNORE,
        ChildOf(root),
    ));

    for (index, label) in labels.into_iter().enumerate() {
        let trigger = commands
            .spawn((
                Button,
                TabTrigger { index },
                Node {
                    height: Val::Px(HEIGHT - INDICATOR_INSET * 2.0),
                    padding: UiRect::horizontal(Val::Px(TRIGGER_PADDING_X)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Interaction::default(),
                crate::cursor::HoverCursor::default(),
                ChildOf(root),
            ))
            .id();

        commands.spawn((
            Text::new(label),
            style.font.text_font(style.typography.sm),
            TextColor(if index == active {
                style.palette.foreground
            } else {
                style.palette.muted_foreground
            }),
            bevy::picking::Pickable::IGNORE,
            ChildOf(trigger),
        ));
    }

    root
}
