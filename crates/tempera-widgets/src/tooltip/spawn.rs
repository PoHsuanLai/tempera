use bevy::prelude::*;

use super::components::{Tooltip, TooltipArrow, TooltipPopup, TooltipPosition};
use crate::theme::{ColorPalette, FontHandle, Step, Tokens, Typography};

/// Half the diagonal of the rotated square that forms the arrow.
///
/// **Off the spacing scale deliberately.** The arrow is a square rotated 45°,
/// so its edge is `ARROW_SIZE * √2` and exactly half of it protrudes past the
/// popup's border while the other half hides behind it (see `spawn_arrow`).
/// The number is fixed by that geometry and by meeting a 1px border cleanly;
/// 4 or 6 leaves a visible seam. It is not a grid value and must not be
/// snapped to one.
pub(crate) const ARROW_SIZE: f32 = 5.0;

/// The tooltip's own geometry, resolved from the spacing scale.
///
/// Steps 1, 3 and 1 — 6, 12 and 6 at the default base, which is what these
/// were written as before the scale could name them.
pub(crate) struct TooltipMetrics {
    pub corner_radius: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    /// Gap between the tooltip's label and a shortcut chip beside it.
    pub gap_between_label_and_chip: f32,
}

impl TooltipMetrics {
    pub(crate) fn from(tokens: &Tokens) -> Self {
        Self {
            corner_radius: tokens.scale.radius_at(Step::new(1)).get(),
            padding_x: tokens.scale.at(Step::new(3)).get(),
            padding_y: tokens.scale.at(Step::new(1)).get(),
            gap_between_label_and_chip: tokens.scale.at(Step::new(2)).get(),
        }
    }
}

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
    metrics: &TooltipMetrics,
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
                padding: UiRect::axes(Val::Px(metrics.padding_x), Val::Px(metrics.padding_y)),
                max_width: Val::Px(tooltip.max_width),
                border_radius: BorderRadius::all(Val::Px(metrics.corner_radius)),
                // Lay text + shortcut chips out as a row so the kbd
                // chips sit to the right of the message (matches
                // shadcn's `<TooltipContent>Save <Kbd>S</Kbd></...>`).
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(metrics.gap_between_label_and_chip),
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

/// Spawn the kbd chips as children of the tooltip popup.
///
/// The popup paints inverted (`bg-foreground`, `text-background`), so the
/// standard muted-on-default cap reads as a smudge on it — hence
/// [`KbdColors::on_inverted`].
///
/// The *colours* are all that differs. This used to be a full fork of
/// `spawn_kbd` carrying its own geometry, which then drifted (5/1 padding
/// against 6/2) and missed the keycap alignment floor entirely — a tooltip's
/// shortcuts were ragged where the same chord in the keybindings tab was
/// not. Calling the shared spawner is what keeps the two in step.
fn spawn_kbd_chips_for_tooltip(
    commands: &mut Commands,
    parent: Entity,
    chord: &crate::kbd::KbdChord,
    palette: &ColorPalette,
    typography: &Typography,
    font: &FontHandle,
) {
    let row = crate::kbd::spawn_kbd_in(
        commands,
        chord.clone(),
        crate::kbd::KbdColors::on_inverted(palette),
        // The popup owns these colours; `repaint_kbd` must not resolve them
        // back to the standard palette.
        crate::kbd::Repaint::CallerOwns,
        font,
        typography,
    );
    commands
        .entity(row)
        .insert((ChildOf(parent), bevy::picking::Pickable::IGNORE));
}

/// Suppress warning — `TooltipPosition` is part of the public API.
#[allow(dead_code)]
const _: fn() = || {
    let _ = TooltipPosition::Auto;
};
