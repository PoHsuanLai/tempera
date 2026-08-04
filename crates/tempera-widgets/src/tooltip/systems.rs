use bevy::picking::events::{Out, Over, Pointer};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::PrimaryWindow;

use super::components::{
    Tooltip, TooltipArrow, TooltipHover, TooltipPopup, TooltipPosition, TooltipShortcutFor,
};
use super::spawn::spawn_popup;
use crate::theme::{ColorPalette, FontHandle, Typography};

/// Z-order for tooltip popups. Above menus, below modals.
pub(crate) const Z_TOOLTIP: i32 = 2000;

/// Write each hovered tooltip's chord from the command it tracks.
///
/// Runs before [`open_tooltips`], so the popup is built from the fresh value.
/// [`TooltipHover`] is inserted by a `Pointer<Over>` observer, so it is already
/// present when `Update` starts and a hover never waits a frame.
///
/// Writing `None` matters as much as writing `Some`: the entity may carry a
/// chord from a previous hover, and a binding cleared since then must clear it.
/// See [`TooltipShortcutFor`] for why this is not resolved once at spawn.
pub(crate) fn resolve_command_shortcuts(
    registry: Option<Res<tempera_input::CommandRegistry>>,
    keybinds: Query<&tempera_input::Keybind>,
    mut hovered: Query<(&TooltipShortcutFor, &mut Tooltip), Added<TooltipHover>>,
) {
    // No registry means no `TemperaInputPlugin`. A tooltip naming a command in
    // an app with no command system shows no chord rather than failing.
    let Some(registry) = registry else { return };

    for (tracks, mut tooltip) in &mut hovered {
        let resolved = registry
            .get(tracks.0.as_str())
            .and_then(|command| keybinds.get(command).ok())
            .map(|bind| bind.0.clone().into());

        if tooltip.shortcut != resolved {
            tooltip.shortcut = resolved;
        }
    }
}

/// On `Pointer<Over>` of a tooltip target, start the hover timer.
pub(crate) fn on_hover_start(
    on: On<Pointer<Over>>,
    time: Res<Time>,
    targets: Query<(), With<Tooltip>>,
    mut commands: Commands,
) {
    if targets.get(on.entity).is_err() {
        return;
    }
    commands.entity(on.entity).insert(TooltipHover {
        started_at: time.elapsed_secs(),
    });
}

/// On `Pointer<Out>`, drop the hover timer and any popup spawned for
/// this target.
pub(crate) fn on_hover_end(
    on: On<Pointer<Out>>,
    targets: Query<(), With<Tooltip>>,
    popups: Query<(Entity, &TooltipPopup)>,
    mut commands: Commands,
) {
    if targets.get(on.entity).is_err() {
        return;
    }
    commands.entity(on.entity).remove::<TooltipHover>();
    for (popup, marker) in &popups {
        if marker.target == on.entity {
            commands.entity(popup).try_despawn();
        }
    }
}

/// Once `delay_ms` has elapsed for a hovered target without a popup
/// already spawned, spawn it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_tooltips(
    time: Res<Time>,
    palette: Res<ColorPalette>,
    typography: Res<Typography>,
    font: Res<FontHandle>,
    tokens: Res<crate::theme::Tokens>,
    hovered: Query<
        (
            Entity,
            &Tooltip,
            &TooltipHover,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        Without<TooltipPopup>,
    >,
    popups: Query<&TooltipPopup>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    let now = time.elapsed_secs();
    for (entity, tooltip, hover, node, transform) in &hovered {
        // Don't double-spawn if one already exists for this target.
        if popups.iter().any(|p| p.target == entity) {
            continue;
        }
        let elapsed_ms = ((now - hover.started_at) * 1000.0) as u64;
        if elapsed_ms < tooltip.delay_ms {
            continue;
        }

        let scale = node.inverse_scale_factor();
        let target_center = transform.translation * scale;
        let target_half = node.size() * scale * 0.5;
        spawn_popup(
            &mut commands,
            entity,
            tooltip,
            target_center,
            target_half,
            window_size,
            &palette,
            &typography,
            &font,
            &super::spawn::TooltipMetrics::from(&tokens),
        );
    }
}

/// Keep the popup anchored to its target each frame. Targets can move
/// (animated layouts, scroll containers) so we recompute every frame
/// while the popup exists. Also despawn orphans whose target is gone.
///
/// On the first frame the popup's `ComputedNode` reports a non-zero
/// size we flip `Visibility::Hidden → Inherited` — that's how we hide
/// the parked-at-target-center first frame from the user (otherwise
/// the popup would visibly flash from the target center to its
/// resolved side once layout measures it).
pub(crate) fn sync_popup_positions(
    targets: Query<(&Tooltip, &ComputedNode, &UiGlobalTransform), Without<TooltipPopup>>,
    popup_size: Query<&ComputedNode, With<TooltipPopup>>,
    mut popups: Query<
        (Entity, &mut TooltipPopup, &mut Node, &mut Visibility),
        Without<TooltipArrow>,
    >,
    mut arrows: Query<(&ChildOf, &mut Node), (With<TooltipArrow>, Without<TooltipPopup>)>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());

    for (popup_entity, mut marker, mut node, mut visibility) in &mut popups {
        let Ok((tooltip, target_node, target_xf)) = targets.get(marker.target) else {
            // Target is gone — despawn the orphaned popup.
            commands.entity(popup_entity).try_despawn();
            continue;
        };
        let Ok(popup_cn) = popup_size.get(popup_entity) else {
            continue;
        };
        let scale = target_node.inverse_scale_factor();
        let popup_size_logical = popup_cn.size() * popup_cn.inverse_scale_factor();
        // Bail until layout has actually measured the popup — flipping
        // to visible with a zero-size measurement would draw at the
        // parked target-center for one frame.
        if popup_size_logical.x < 1.0 || popup_size_logical.y < 1.0 {
            continue;
        }
        let target_center = target_xf.translation * scale;
        let target_half = target_node.size() * scale * 0.5;

        let placement = place_tooltip(
            tooltip.position,
            target_center,
            target_half,
            popup_size_logical,
            window_size,
            tooltip.show_arrow,
        );
        node.left = Val::Px(placement.popup_min.x);
        node.top = Val::Px(placement.popup_min.y);
        marker.position = placement.resolved;
        // Reveal — measured + positioned this frame.
        if *visibility == Visibility::Hidden {
            *visibility = Visibility::Inherited;
        }

        // Reposition the arrow child to point at the target.
        for (child_of, mut arrow_node) in &mut arrows {
            if child_of.0 != popup_entity {
                continue;
            }
            let (arrow_x, arrow_y, arrow_rotation) =
                arrow_offset(placement.resolved, popup_size_logical);
            arrow_node.left = Val::Px(arrow_x);
            arrow_node.top = Val::Px(arrow_y);
            // We use a single triangle texture; rotate via transform.
            // (Rotation handled in spawn via 4 separate triangle
            // shapes — see super::spawn.)
            let _ = arrow_rotation;
        }
    }
}

pub(crate) struct Placement {
    pub popup_min: Vec2,
    pub resolved: TooltipPosition,
}

pub(crate) fn place_tooltip(
    preferred: TooltipPosition,
    target_center: Vec2,
    target_half: Vec2,
    popup_size: Vec2,
    window_size: Vec2,
    show_arrow: bool,
) -> Placement {
    let arrow_gap = if show_arrow {
        super::spawn::ARROW_SIZE + 2.0
    } else {
        4.0
    };

    let resolved = match preferred {
        TooltipPosition::Auto => choose_auto(
            target_center,
            target_half,
            popup_size,
            window_size,
            arrow_gap,
        ),
        explicit => explicit,
    };

    let popup_min = match resolved {
        TooltipPosition::Top => Vec2::new(
            target_center.x - popup_size.x * 0.5,
            target_center.y - target_half.y - popup_size.y - arrow_gap,
        ),
        TooltipPosition::Bottom => Vec2::new(
            target_center.x - popup_size.x * 0.5,
            target_center.y + target_half.y + arrow_gap,
        ),
        TooltipPosition::Left => Vec2::new(
            target_center.x - target_half.x - popup_size.x - arrow_gap,
            target_center.y - popup_size.y * 0.5,
        ),
        TooltipPosition::Right => Vec2::new(
            target_center.x + target_half.x + arrow_gap,
            target_center.y - popup_size.y * 0.5,
        ),
        TooltipPosition::Auto => unreachable!("resolved above"),
    };

    // Clamp into the window so the popup is fully visible.
    let popup_min = Vec2::new(
        popup_min
            .x
            .clamp(4.0, (window_size.x - popup_size.x - 4.0).max(4.0)),
        popup_min
            .y
            .clamp(4.0, (window_size.y - popup_size.y - 4.0).max(4.0)),
    );

    Placement {
        popup_min,
        resolved,
    }
}

fn choose_auto(
    target_center: Vec2,
    target_half: Vec2,
    popup_size: Vec2,
    window_size: Vec2,
    gap: f32,
) -> TooltipPosition {
    let space_above = target_center.y - target_half.y;
    let space_below = window_size.y - (target_center.y + target_half.y);
    let space_left = target_center.x - target_half.x;
    let space_right = window_size.x - (target_center.x + target_half.x);

    let need_v = popup_size.y + gap;
    let need_h = popup_size.x + gap;

    if space_above >= need_v {
        TooltipPosition::Top
    } else if space_below >= need_v {
        TooltipPosition::Bottom
    } else if space_right >= need_h {
        TooltipPosition::Right
    } else if space_left >= need_h {
        TooltipPosition::Left
    } else {
        TooltipPosition::Top
    }
}

/// Pixel offset (left, top) of the diamond-rotated arrow child within
/// the popup. The arrow is a square of edge `edge = ARROW_SIZE * √2`
/// rotated 45° via `UiTransform`, so its visible corner protrudes
/// `ARROW_SIZE` past the popup edge while the back half hides behind
/// the popup's rounded rect. We position the **center** of the
/// unrotated rect on the popup edge (rotation pivots around the
/// rect's center), which means `left/top = center - edge/2`.
fn arrow_offset(position: TooltipPosition, popup_size: Vec2) -> (f32, f32, f32) {
    let edge = super::spawn::ARROW_SIZE * std::f32::consts::SQRT_2;
    let half = edge * 0.5;
    match position {
        TooltipPosition::Top => (popup_size.x * 0.5 - half, popup_size.y - half, 0.0),
        TooltipPosition::Bottom => (popup_size.x * 0.5 - half, -half, 0.0),
        TooltipPosition::Left => (popup_size.x - half, popup_size.y * 0.5 - half, 0.0),
        TooltipPosition::Right => (-half, popup_size.y * 0.5 - half, 0.0),
        TooltipPosition::Auto => (0.0, 0.0, 0.0),
    }
}
