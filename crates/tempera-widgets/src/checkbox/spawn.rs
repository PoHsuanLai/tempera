use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui_widgets::Checkbox;

use super::components::{CheckGlyph, TemperaCheckbox};
use crate::anim::Spring;
use crate::theme::{ColorPalette, FontHandle, Typography};

const BOX_SIZE: f32 = 16.0;
const CORNER: f32 = 4.0;
const BORDER: f32 = 1.0;

/// Tokens read by checkbox systems.
#[derive(SystemParam)]
pub struct CheckboxStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
}

/// Spawn a styled checkbox. Returns the root entity (the [`Checkbox`]
/// owner). Observe `ValueChange<bool>` on it to react to flips.
pub fn spawn_checkbox(commands: &mut Commands, style: &CheckboxStyle, checked: bool) -> Entity {
    let initial_t = if checked { 1.0 } else { 0.0 };
    let mut root = commands.spawn((
        Checkbox,
        TemperaCheckbox,
        // Spring drives the check glyph's scale (0 = hidden, 1 = full
        // size). armas's checkbox spring is firmer than the switch's
        // so the checkmark pops in crisply rather than easing.
        Spring::new(initial_t, initial_t).params(2000.0, 55.0),
        Node {
            width: Val::Px(BOX_SIZE),
            height: Val::Px(BOX_SIZE),
            border: UiRect::all(Val::Px(BORDER)),
            border_radius: BorderRadius::all(Val::Px(CORNER)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(if checked {
            style.palette.primary
        } else {
            style.palette.background
        }),
        BorderColor::all(style.palette.input),
        Interaction::default(),
        crate::cursor::HoverCursor::default(),
        Name::new("tempera::checkbox"),
    ));

    if checked {
        root.insert(bevy::ui::Checked);
    }
    let id = root.id();

    // Check glyph — always `Display::Flex`. The drive system animates
    // its `Transform.scale` between 0 and 1 so the glyph springs in /
    // out instead of snapping. TextColor alpha is masked to 0 below
    // ~0.05 so the spawn frame doesn't briefly flash a full-size
    // glyph before the scale arrives.
    let initial_alpha = if checked { 1.0 } else { 0.0 };
    let mut glyph_color = style.palette.primary_foreground;
    glyph_color.set_alpha(initial_alpha);
    commands.spawn((
        CheckGlyph,
        Text::new("✓"),
        style.font.text_font(style.typography.sm),
        TextColor(glyph_color),
        Node::default(),
        Transform::from_scale(Vec3::splat(initial_t)),
        bevy::picking::Pickable::IGNORE,
        ChildOf(id),
    ));

    id
}
