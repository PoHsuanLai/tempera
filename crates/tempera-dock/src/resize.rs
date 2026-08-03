//! Dragging a divider.
//!
//! One observer serves every divider in every split, because a divider finds
//! the nodes it resizes **by position** — the children on either side of it in
//! its parent's child list — rather than by marker. A layout where one seam
//! resizes against a sibling and another against the window needs two
//! implementations and cannot be rearranged; this one needs neither.

use bevy::picking::events::{Drag, Out, Over, Pointer};
use bevy::prelude::*;
use tempera_theme::ColorPalette;

use crate::node::{Axis, Divider, PaneMinSize, PaneSize, Split};

/// Divider thickness and hover tint.
///
/// One resource rather than per-divider components: a dock with two divider
/// styles is a dock with a design problem, and the seam between panes is
/// chrome the *shell* owns, not something a pane should be able to restyle.
#[derive(Resource, Debug, Clone)]
pub struct DividerStyle {
    /// Grab width in logical pixels. Wide enough to hit, narrow enough to read
    /// as a seam rather than a gap.
    pub thickness: f32,
    /// Painted while hovered or dragging. `None` leaves dividers invisible,
    /// which is the right default over a 3D viewport.
    pub hover: Option<Color>,
}

impl Default for DividerStyle {
    fn default() -> Self {
        Self {
            thickness: 8.0,
            hover: None,
        }
    }
}

/// Redistribute flex weight between two adjacent nodes.
///
/// `delta_px` is how far the seam moved, positive meaning `prev` grows. Total
/// weight is conserved, which is what keeps a third sibling's size unchanged
/// when its neighbours resize against each other.
///
/// Public because it is the whole of the resize maths and is worth testing
/// without a `World`.
pub fn redistribute_flex(
    prev_grow: f32,
    prev_size_px: f32,
    next_grow: f32,
    next_size_px: f32,
    delta_px: f32,
) -> (f32, f32) {
    let total_size = prev_size_px + next_size_px;
    if total_size <= 0.0 {
        return (prev_grow, next_grow);
    }
    let new_prev = (prev_size_px + delta_px).clamp(0.0, total_size);
    let new_next = total_size - new_prev;
    let total_grow = prev_grow + next_grow;
    let prev_g = total_grow * new_prev / total_size;
    let next_g = total_grow * new_next / total_size;
    (prev_g, next_g)
}

/// Clamp a drag so neither side lands under its [`PaneMinSize`].
///
/// Separate from [`redistribute_flex`] because the clamp is about the *panes*
/// and the redistribution is about the *weights*; keeping them apart is what
/// lets the maths above stay a pure function of five floats.
pub(crate) fn clamp_delta(
    delta: f32,
    prev_size: f32,
    next_size: f32,
    prev_min: Option<f32>,
    next_min: Option<f32>,
) -> f32 {
    // Negative shrinks `prev` and grows `next`; positive does the reverse. So
    // each pane's floor bounds exactly one direction — the one that shrinks it.
    //
    // A pane can already sit under its own minimum: the window shrank, or the
    // minimum was raised since the layout was saved. Its bound is then on the
    // wrong side of zero, and clamping into it would *force* movement on a drag
    // that should simply be refused. Hence `min(0.0)` / `max(0.0)`: a floor may
    // shrink the allowed range down to nothing, never past it into a shove.
    let mut lo = -prev_size;
    let mut hi = next_size;
    if let Some(min) = prev_min {
        lo = lo.max((min - prev_size).min(0.0));
    }
    if let Some(min) = next_min {
        hi = hi.min((next_size - min).max(0.0));
    }
    delta.clamp(lo.min(0.0), hi.max(0.0))
}

/// Per-node facts the drag needs, read before anything is written.
struct Side {
    entity: Entity,
    size_px: f32,
    grow: f32,
    fixed: bool,
    min: Option<f32>,
}

/// Resize the two nodes flanking the dragged divider.
pub(crate) fn on_divider_drag(
    drag: On<Pointer<Drag>>,
    dividers: Query<&ChildOf, With<Divider>>,
    splits: Query<(&Split, &Children)>,
    mut nodes: Query<(
        &mut PaneSize,
        Option<&PaneMinSize>,
        &mut Node,
        &ComputedNode,
    )>,
) {
    let divider = drag.entity;
    let Ok(parent) = dividers.get(divider) else {
        return;
    };
    let Ok((split, children)) = splits.get(parent.parent()) else {
        return;
    };
    let Some(index) = children.iter().position(|c| c == divider) else {
        return;
    };
    // A divider is only ever interleaved between two content nodes, so both
    // neighbours exist whenever the tree is well-formed.
    let (Some(prev), Some(next)) = (
        index.checked_sub(1).and_then(|i| children.get(i)).copied(),
        children.get(index + 1).copied(),
    ) else {
        return;
    };

    let Some((prev, next)) = read_sides(&nodes, split.axis, prev, next) else {
        return;
    };
    if prev.fixed && next.fixed {
        // Two fixed neighbours have no weight to trade; moving the seam would
        // have to resize the whole split, which is the parent's business.
        return;
    }

    let delta = clamp_delta(
        split.axis.delta_along(drag.event.delta),
        prev.size_px,
        next.size_px,
        prev.min,
        next.min,
    );
    if delta == 0.0 {
        return;
    }

    match (prev.fixed, next.fixed) {
        (false, false) => {
            let (prev_grow, next_grow) =
                redistribute_flex(prev.grow, prev.size_px, next.grow, next.size_px, delta);
            write_flex(&mut nodes, prev.entity, prev_grow);
            write_flex(&mut nodes, next.entity, next_grow);
        }
        // One side is fixed: it absorbs the whole delta in pixels and the flex
        // side takes up the slack by construction. Writing both would
        // double-count the movement.
        (true, false) => write_fixed(&mut nodes, split.axis, prev.entity, prev.size_px + delta),
        (false, true) => write_fixed(&mut nodes, split.axis, next.entity, next.size_px - delta),
        (true, true) => unreachable!("both-fixed returns above"),
    }
}

fn read_sides(
    nodes: &Query<(
        &mut PaneSize,
        Option<&PaneMinSize>,
        &mut Node,
        &ComputedNode,
    )>,
    axis: Axis,
    prev: Entity,
    next: Entity,
) -> Option<(Side, Side)> {
    let read = |entity: Entity| {
        let (size, min, _, computed) = nodes.get(entity).ok()?;
        Some(Side {
            entity,
            // `ComputedNode` is in physical pixels; every extent the dock
            // reasons about is logical, so scale on the way in.
            size_px: axis.extent_of(computed.size()) * computed.inverse_scale_factor,
            grow: size.flex().unwrap_or(0.0),
            fixed: size.flex().is_none(),
            min: min.map(|m| m.0),
        })
    };
    Some((read(prev)?, read(next)?))
}

fn write_flex(
    nodes: &mut Query<(
        &mut PaneSize,
        Option<&PaneMinSize>,
        &mut Node,
        &ComputedNode,
    )>,
    entity: Entity,
    grow: f32,
) {
    if let Ok((mut size, _, mut node, _)) = nodes.get_mut(entity) {
        *size = PaneSize::Flex(grow);
        node.flex_grow = grow;
    }
}

fn write_fixed(
    nodes: &mut Query<(
        &mut PaneSize,
        Option<&PaneMinSize>,
        &mut Node,
        &ComputedNode,
    )>,
    axis: Axis,
    entity: Entity,
    px: f32,
) {
    let px = px.max(0.0);
    if let Ok((mut size, _, mut node, _)) = nodes.get_mut(entity) {
        *size = PaneSize::Fixed(px);
        match axis {
            Axis::Row => node.width = Val::Px(px),
            Axis::Column => node.height = Val::Px(px),
        }
    }
}

/// Tint a divider while the pointer is over it.
pub(crate) fn on_divider_hover(
    over: On<Pointer<Over>>,
    style: Res<DividerStyle>,
    palette: Option<Res<ColorPalette>>,
    mut dividers: Query<&mut BackgroundColor, With<Divider>>,
) {
    let Ok(mut bg) = dividers.get_mut(over.entity) else {
        return;
    };
    if let Some(color) = hover_tint(&style, palette.as_deref()) {
        bg.0 = color;
    }
}

/// Drop the hover tint when the pointer leaves.
pub(crate) fn on_divider_out(
    out: On<Pointer<Out>>,
    mut dividers: Query<&mut BackgroundColor, With<Divider>>,
) {
    if let Ok(mut bg) = dividers.get_mut(out.entity) {
        bg.0 = Color::NONE;
    }
}

/// An explicit tint wins; otherwise step the palette's border colour away from
/// the surface it is drawn on, enough to read as a seam. With no palette at
/// all the divider stays invisible, which is correct over a viewport the shell
/// must not paint over.
///
/// The surface is `background`: a divider separates panels, and what it has to
/// stand out against is the shell behind them.
fn hover_tint(style: &DividerStyle, palette: Option<&ColorPalette>) -> Option<Color> {
    style
        .hover
        .or_else(|| palette.map(|p| ColorPalette::step(p.border, p.background, 0.15)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redistribute_conserves_total_weight() {
        let (a, b) = redistribute_flex(1.0, 100.0, 6.0, 600.0, 50.0);
        assert!((a + b - 7.0).abs() < 1e-5, "total weight must be conserved");
        assert!(a > 1.0, "prev grew");
        assert!(b < 6.0, "next shrank");
    }

    #[test]
    fn a_pane_can_be_dragged_to_zero() {
        // No implicit floor: a pane the user can squash out of the way is the
        // default, and `PaneMinSize` is how a pane opts out.
        let (a, b) = redistribute_flex(1.0, 100.0, 6.0, 600.0, -500.0);
        assert_eq!(a, 0.0);
        assert!((b - 7.0).abs() < 1e-5, "the neighbour takes it all");
    }

    #[test]
    fn a_drag_past_the_far_edge_is_clamped() {
        let (a, b) = redistribute_flex(1.0, 100.0, 6.0, 600.0, 5_000.0);
        assert!((a - 7.0).abs() < 1e-5);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn zero_total_size_is_a_no_op() {
        // Before the first layout pass every node measures zero. Redistributing
        // then would divide by zero and poison both weights.
        assert_eq!(redistribute_flex(1.0, 0.0, 6.0, 0.0, 25.0), (1.0, 6.0));
    }

    #[test]
    fn dragging_two_panes_does_not_move_a_third() {
        // The property that makes a 3-pane row work: the pair trades weight
        // between themselves, so the untouched sibling keeps its share of the
        // total.
        let (browser, center) = (1.0_f32, 6.0_f32);
        let inspector = 2.0_f32;
        let before_total = browser + center + inspector;

        let (browser, center) = redistribute_flex(browser, 100.0, center, 600.0, 40.0);

        let after_total = browser + center + inspector;
        assert!((before_total - after_total).abs() < 1e-5);
        assert!((inspector / after_total - 2.0 / before_total).abs() < 1e-5);
    }

    #[test]
    fn min_size_stops_the_drag() {
        assert_eq!(clamp_delta(-50.0, 300.0, 300.0, Some(280.0), None), -20.0);
    }

    #[test]
    fn a_pane_already_under_its_minimum_is_not_snapped_outward() {
        // The window shrank, or the minimum was raised since the layout was
        // saved. A shrink must be refused — but refusing means *not moving*.
        // A naive clamp turns the floor into a lower bound above zero and
        // shoves the pane outward mid-drag.
        assert_eq!(clamp_delta(-50.0, 100.0, 300.0, Some(120.0), None), 0.0);
        assert_eq!(clamp_delta(50.0, 300.0, 100.0, None, Some(120.0)), 0.0);
    }

    #[test]
    fn an_under_minimum_pane_can_still_grow() {
        // The floor bounds only the direction that shrinks the pane it
        // protects. A pane below its minimum must stay draggable back toward
        // legality, or it is stuck the moment it gets there.
        assert_eq!(clamp_delta(40.0, 100.0, 300.0, Some(120.0), None), 40.0);
        assert_eq!(clamp_delta(-40.0, 300.0, 100.0, None, Some(120.0)), -40.0);
    }

    #[test]
    fn min_size_on_the_far_side_caps_a_grow() {
        assert_eq!(clamp_delta(200.0, 300.0, 300.0, None, Some(200.0)), 100.0);
    }

    #[test]
    fn without_a_minimum_a_drag_is_bounded_only_by_the_pair() {
        assert_eq!(clamp_delta(-500.0, 300.0, 300.0, None, None), -300.0);
        assert_eq!(clamp_delta(500.0, 300.0, 300.0, None, None), 300.0);
        assert_eq!(clamp_delta(25.0, 300.0, 300.0, None, None), 25.0);
    }

    #[test]
    fn dividers_are_invisible_without_a_palette_or_an_override() {
        // The dock sits over a 3D viewport in at least one host; painting a
        // seam it was never told to paint would occlude it.
        let style = DividerStyle::default();
        assert_eq!(hover_tint(&style, None), None);

        let style = DividerStyle {
            hover: Some(Color::WHITE),
            ..default()
        };
        assert_eq!(hover_tint(&style, None), Some(Color::WHITE));
    }
}
