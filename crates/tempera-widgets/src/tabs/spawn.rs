use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use super::components::{TabIndicator, TabTrigger, Tabs, TabsActive};
use crate::theme::{
    ColorPalette, ControlSize, FontHandle, Metrics, Spacing, Step, StyledNode, Typography,
};

// Matches the dawai browser default: HEIGHT=26, TRIGGER_PADDING_X=8,
// INDICATOR_INSET=2. The indicator is a solid `background`-filled
// rounded rect (radius 4) inset by 2px on every side, matching
// shadcn/ui Tabs and `armas-basic::Tabs`. (armas default is 28 but
// dawai's panels all override to 26 — picking the smaller as the
// tempera default eliminates the per-call override.)

/// Gap between the strip's edge and the indicator inside it.
///
/// Read from the scale (step −2) at spawn, but kept as a const because
/// `move_indicator` needs the same number every frame to position the
/// indicator, and the two must agree *exactly* or the indicator sits crooked
/// against the container padding it is supposed to nest inside. That is a
/// relationship between two values rather than a grid value, so it is
/// declared once and shared rather than looked up twice.
pub(crate) const INDICATOR_INSET: f32 = 2.0;

#[derive(SystemParam)]
pub struct TabsStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub metrics: Metrics<'w>,
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
            StyledNode::new()
                .height(ControlSize::Sm)
                .radius(Step::new(1)),
            TabsActive(active),
            Node {
                flex_direction: FlexDirection::Row,
                padding: UiRect::all(Val::Px(INDICATOR_INSET)),
                position_type: PositionType::Relative,
                // Full-width by default — every trigger flex-grows to
                // share the row equally (matches armas-basic Tabs and
                // shadcn/ui's default `inline-flex w-full` shape). To
                // shrink-to-content, override `align_self` and
                // `width` on the returned root.
                width: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(style.palette.muted),
            Name::new("tempera::tabs"),
        ))
        .id();

    // Active indicator — a `background`-filled rounded rect inset by
    // 2px on every side, matching armas-basic Tabs / shadcn v4 Tabs
    // (Tailwind: `bg-background`, `rounded-md`). The paint system
    // (`move_indicator`) reads each trigger's `ComputedNode` and
    // moves this Node's `left` + `width` under the active trigger.
    commands.spawn((
        TabIndicator,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(INDICATOR_INSET),
            left: Val::Px(INDICATOR_INSET),
            width: Val::Px(0.0),
            height: Val::Px(style.metrics.control(ControlSize::Sm).get() - INDICATOR_INSET * 2.0),
            border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_tiny)),
            ..default()
        },
        BackgroundColor(style.palette.background),
        bevy::picking::Pickable::IGNORE,
        ChildOf(root),
    ));

    for (index, label) in labels.into_iter().enumerate() {
        let trigger = commands
            .spawn((
                Button,
                TabTrigger { index },
                Node {
                    // Each trigger grows to share the row's width
                    // evenly. Container padding (INDICATOR_INSET) is
                    // already on the root, so triggers fill the
                    // remaining inner row.
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    height: Val::Px(
                        style.metrics.control(ControlSize::Sm).get() - INDICATOR_INSET * 2.0,
                    ),
                    padding: UiRect::horizontal(Val::Px(style.metrics.gap(Step::new(2)).get())),
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
