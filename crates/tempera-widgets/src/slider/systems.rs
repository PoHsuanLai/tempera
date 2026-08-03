//! Paint systems — position the thumb + fill, retint on hover/disabled.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled};
use bevy::ui_widgets::{Slider, SliderRange, SliderThumb, SliderValue};

use super::components::{SliderFill, SliderSize, SliderTrack};
use crate::theme::ColorPalette;

/// Read-side projection over a slider entity used by paint systems.
#[derive(QueryData)]
pub(crate) struct SliderState {
    value: &'static SliderValue,
    range: &'static SliderRange,
    size: Option<&'static SliderSize>,
}

/// Move the thumb's `left` and stretch the fill to match the current
/// [`SliderValue`] on its parent slider entity.
///
/// **Coordinate space:** Bevy's headless slider does its drag math in
/// **logical** pixels (it divides cursor delta by `UiScale` and the
/// slider width by `inverse_scale_factor`). `Node` field values like
/// `Val::Px` are also logical. But `ComputedNode::size()` returns
/// **physical** pixels. So we must multiply by `inverse_scale_factor`
/// to convert before writing back to `Val::Px` — otherwise on a 2x
/// (Retina) display the thumb visually moves at twice the cursor's
/// speed.
pub(crate) fn reposition_thumb(
    sliders: Query<SliderState, With<Slider>>,
    tracks: Query<(&ComputedNode, &ChildOf), With<SliderTrack>>,
    mut thumbs: Query<(&mut Node, &ChildOf), (With<SliderThumb>, Without<SliderFill>)>,
    mut fills: Query<(&mut Node, &ChildOf), (With<SliderFill>, Without<SliderThumb>)>,
) {
    for (mut node, parent) in &mut thumbs {
        let Ok((track_node, track_parent)) = tracks.get(parent.0) else {
            continue;
        };
        let Ok(state) = sliders.get(track_parent.0) else {
            continue;
        };
        let pos = state.range.thumb_position(state.value.0);
        let size = state.size.copied().unwrap_or_default();
        let track_w_logical = track_node.size().x * track_node.inverse_scale_factor;
        let travel = (track_w_logical - size.thumb_diameter()).max(0.0);
        node.left = Val::Px(pos * travel);
    }

    for (mut node, parent) in &mut fills {
        let Ok((track_node, track_parent)) = tracks.get(parent.0) else {
            continue;
        };
        let Ok(state) = sliders.get(track_parent.0) else {
            continue;
        };
        let pos = state.range.thumb_position(state.value.0);
        let size = state.size.copied().unwrap_or_default();
        let track_w_logical = track_node.size().x * track_node.inverse_scale_factor;
        let travel = (track_w_logical - size.thumb_diameter()).max(0.0);
        let fill_end_px = pos * travel + size.thumb_diameter() * 0.5;
        node.width = Val::Px(fill_end_px);
    }
}

/// Retint the fill + thumb when the slider's hover or disabled state
/// changes. The thumb's border thickness also bumps on hover for a
/// subtle "this is grabbable" cue.
pub(crate) fn repaint_slider(
    palette: Res<ColorPalette>,
    sliders: Query<(Entity, &Interaction, Has<InteractionDisabled>), With<Slider>>,
    children_of: Query<&Children>,
    tracks: Query<&Children, With<SliderTrack>>,
    mut fills: Query<&mut BackgroundColor, (With<SliderFill>, Without<SliderThumb>)>,
    mut thumb_borders: Query<&mut BorderColor, With<SliderThumb>>,
    mut thumb_nodes: Query<&mut Node, With<SliderThumb>>,
) {
    for (slider, interaction, disabled) in &sliders {
        let hovered =
            !disabled && matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        let Ok(kids) = children_of.get(slider) else {
            continue;
        };
        for child in kids.iter() {
            let Ok(track_kids) = tracks.get(child) else {
                continue;
            };
            for grand in track_kids.iter() {
                let color = if disabled {
                    palette.muted_foreground
                } else if hovered {
                    ColorPalette::step(palette.primary, palette.background, 0.08)
                } else {
                    palette.primary
                };
                if let Ok(mut bg) = fills.get_mut(grand)
                    && bg.0 != color
                {
                    *bg = BackgroundColor(color);
                }
                let want_border = BorderColor::all(color);
                if let Ok(mut border) = thumb_borders.get_mut(grand)
                    && *border != want_border
                {
                    *border = want_border;
                }
                // `Node` especially: bevy_ui gates its taffy upload on
                // `Ref<Node>::is_changed()`, and a `DerefMut` sets that
                // flag whether or not the value moved. Writing this
                // unconditionally would re-upload the thumb every frame.
                let want_edge = UiRect::all(Val::Px(if hovered { 3.0 } else { 2.0 }));
                if let Ok(mut node) = thumb_nodes.get_mut(grand)
                    && node.border != want_edge
                {
                    node.border = want_edge;
                }
            }
        }
    }
}
