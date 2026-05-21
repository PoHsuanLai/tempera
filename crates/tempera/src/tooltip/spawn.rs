use bevy::prelude::*;

use super::components::{Tooltip, TooltipArrow, TooltipPopup, TooltipPosition};
use crate::kbd::KbdKey;
use crate::theme::{ColorPalette, FontHandle, Typography};

pub(crate) const ARROW_SIZE: f32 = 5.0;
pub(crate) const CORNER_RADIUS: f32 = 6.0;
pub(crate) const PADDING_X: f32 = 12.0;
pub(crate) const PADDING_Y: f32 = 6.0;

/// Spawn the tooltip popup for `target`. Position is computed by
/// the sync system on the next frame; we drop the popup at the
/// target center initially. Sets `TooltipPopup { target }` so the
/// hover-loss observer can find it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_popup(
    commands: &mut Commands,
    target: Entity,
    tooltip: &Tooltip,
    target_center: Vec2,
    _target_half: Vec2,
    _window_size: Vec2,
    palette: &ColorPalette,
    typography: &Typography,
    font: &FontHandle,
) -> Entity {
    // shadcn tooltip is inverted: bg-foreground, text-background.
    let bg = palette.foreground;
    let fg = palette.background;
    let text_font = font.text_font(typography.sm);

    let popup = commands
        .spawn((
            TooltipPopup {
                target,
                position: tooltip.position,
            },
            Node {
                position_type: PositionType::Absolute,
                // Park off-screen until `sync_popup_positions` measures
                // the popup's ComputedNode and writes the correct
                // position. We stay `Visibility::Hidden` until then so
                // the user never sees the parked frame.
                left: Val::Px(target_center.x),
                top: Val::Px(target_center.y),
                padding: UiRect::axes(Val::Px(PADDING_X), Val::Px(PADDING_Y)),
                max_width: Val::Px(tooltip.max_width),
                border_radius: BorderRadius::all(Val::Px(CORNER_RADIUS)),
                // Lay text + shortcut chips out as a row so the kbd
                // chips sit to the right of the message (matches
                // shadcn's `<TooltipContent>Save <Kbd>S</Kbd></...>`).
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            // Spawn invisible — the sync system flips this to
            // `Visibility::Inherited` on the first frame the position
            // is correctly computed. Avoids a one-frame flash at the
            // target center followed by a jump to the resolved side.
            Visibility::Hidden,
            BackgroundColor(bg),
            GlobalZIndex(super::systems::Z_TOOLTIP),
            bevy::picking::Pickable::IGNORE,
            Name::new("tempera::tooltip::popup"),
        ))
        .id();

    commands.spawn((
        Text::new(tooltip.text.clone()),
        text_font,
        TextColor(fg),
        bevy::picking::Pickable::IGNORE,
        ChildOf(popup),
    ));

    if let Some(chord) = &tooltip.shortcut {
        spawn_kbd_chips_for_tooltip(commands, popup, chord, palette, typography, font);
    }

    if tooltip.show_arrow {
        // True triangle: a colored square rotated 45° so its corner
        // protrudes from the popup edge. Half the diamond is hidden
        // behind the popup; the visible half reads as a right-angle
        // triangle pointing at the target. The sync system places it
        // each frame so the protruding corner aligns with the
        // tooltip-target axis.
        //
        // We use the diagonal length `s = ARROW_SIZE * √2` for the
        // square edge so the protruding half matches `ARROW_SIZE` on
        // each axis (same visual footprint as the old flat tab).
        let edge = ARROW_SIZE * std::f32::consts::SQRT_2;
        commands.spawn((
            TooltipArrow,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(edge),
                height: Val::Px(edge),
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            UiTransform::from_rotation(Rot2::FRAC_PI_4),
            BackgroundColor(bg),
            bevy::picking::Pickable::IGNORE,
            ChildOf(popup),
        ));
    }

    popup
}

/// Spawn the kbd chips as children of the tooltip popup. The popup
/// uses inverted colors (`bg-foreground`, `text-background`), so the
/// standard `spawn_kbd` (muted-on-default) doesn't read well on it.
/// Instead we paint chips with a translucent background-tinted fill
/// and `background`-colored text so they sit lightly on top of the
/// popup's light surface.
fn spawn_kbd_chips_for_tooltip(
    commands: &mut Commands,
    parent: Entity,
    chord: &crate::kbd::KbdChord,
    palette: &ColorPalette,
    typography: &Typography,
    font: &FontHandle,
) {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(2.0),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
            bevy::picking::Pickable::IGNORE,
        ))
        .id();

    let mut chip_bg = palette.background;
    chip_bg.set_alpha(0.18);
    let mut border = palette.background;
    border.set_alpha(0.32);

    for segment in chord.render_order() {
        let display = match segment {
            KbdKey::Modifier(m) => crate::kbd::modifier_glyph(*m).to_string(),
            KbdKey::Key(k) => crate::kbd::key_glyph(*k),
        };
        spawn_chip(
            commands,
            row,
            &display,
            chip_bg,
            border,
            palette.background,
            font,
            typography,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_chip(
    commands: &mut Commands,
    row: Entity,
    display: &str,
    bg: Color,
    border: Color,
    text: Color,
    font: &FontHandle,
    typography: &Typography,
) {
    commands
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            ChildOf(row),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(display.to_string()),
                font.text_font(typography.xs),
                TextColor(text),
                bevy::picking::Pickable::IGNORE,
            ));
        });
}

/// Suppress warning — `TooltipPosition` is part of the public API.
#[allow(dead_code)]
const _: fn() = || {
    let _ = TooltipPosition::Auto;
};
