//! The live tree, as components.
//!
//! [`tree`](crate::tree) is what a host *declares*; this is what the dock
//! *spawns*. The split is deliberate: the declared form is plain serializable
//! data with no `Entity` in it, so the same type round-trips to disk and can
//! be edited without a `World`.
//!
//! Every entity the dock owns carries [`DockNode`], so a host can find or tear
//! down the whole managed tree without knowing which kind each node is.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Marker on every entity the dock spawns — panes, splits and dividers alike.
///
/// Exists so "everything the dock owns" is one query. Without it, tearing down
/// a tree means knowing the closed set of node kinds, which stops being true
/// the moment a `Tabs` variant lands.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DockNode;

/// Marker on the single root of a dock tree.
///
/// Also the natural parent for modal overlays: it is the one node guaranteed
/// to span the window and to outlive every layout change.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DockRoot;

/// A leaf of the tree — the **pane frame**.
///
/// Content is a *child* of this entity, never this entity itself. That
/// separation is what makes a future tab-move one reparent: the frame moves
/// and its content subtree follows, with no content crate involved and no
/// re-spawn. Were content to live on the frame, moving a pane would mean
/// relocating the content and losing whatever state it had accumulated.
///
/// The frame is `position: relative` with `overflow: clip`, so an application
/// can anchor an absolutely-positioned overlay to any pane.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Pane;

/// A branch of the tree.
///
/// Its `Children` alternate content nodes with the [`Divider`]s the builder
/// inserts between them: `[node, divider, node, divider, node]`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Split {
    /// Which way this split divides its children.
    pub axis: Axis,
}

/// Stable identity for one pane.
///
/// A string rather than a marker type because the pane's *content* lives in a
/// crate the dock cannot name, and the dock is not nameable from there either.
/// Owned rather than `&'static str` so an extension can mint an id at runtime.
///
/// This is also the persistence key, which is why an unrecognised id is
/// survivable: a saved layout naming a pane the running build no longer
/// declares drops that pane rather than failing the load.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PaneId(pub String);

impl PaneId {
    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for PaneId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// How much of its parent [`Split`] a node claims.
///
/// Two shapes, and the absence of a third is the point:
///
/// - [`Flex`](PaneSize::Flex) is a weight, redistributed against the *adjacent
///   sibling* when a divider is dragged. Flex panes can be dragged down to
///   zero and back unless they carry a [`PaneMinSize`].
/// - [`Fixed`](PaneSize::Fixed) is logical pixels, for chrome that must not
///   breathe — a title bar, a status strip.
///
/// There is no "fixed with a min and a max". A pane resizable only inside a
/// band is a pane whose neighbour cannot absorb the difference, which is the
/// asymmetry that forces a second resize path to exist. A flex pane is bounded
/// by its neighbour; that is the whole mechanism.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PaneSize {
    /// A flex weight, relative to siblings in the same split.
    Flex(f32),
    /// A fixed extent in logical pixels.
    Fixed(f32),
}

impl Default for PaneSize {
    fn default() -> Self {
        Self::Flex(1.0)
    }
}

impl PaneSize {
    /// The flex weight, or `None` for a fixed node.
    pub fn flex(self) -> Option<f32> {
        match self {
            Self::Flex(w) => Some(w),
            Self::Fixed(_) => None,
        }
    }
}

/// Floor, in logical pixels, below which a drag will not shrink this node.
///
/// Absent means the node may collapse to zero — the default, because a pane
/// the user can squash out of the way is usually what they want. Present when
/// a pane has content that is meaningless below some width.
///
/// An absent component rather than an `Option` field: optional behaviour is
/// modelled by not being there.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PaneMinSize(pub f32);

/// Whether a node takes part in layout.
///
/// Hiding sets `Display::None` on the node **and** on the divider that would
/// otherwise be left dangling beside it, so a hidden pane leaves no seam.
///
/// `Display`, not `Visibility`: a hidden pane must measure zero, because hosts
/// read `ComputedNode` sizes to position chrome against a pane's edge. A
/// `Visibility::Hidden` node still occupies its space and would strand that
/// chrome in mid-air.
///
/// [`PaneSize`] is untouched by hiding, so unhiding restores the size the user
/// last dragged to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PaneVisibility {
    /// Takes part in layout.
    #[default]
    Shown,
    /// `Display::None`, and its adjacent divider with it.
    Hidden,
}

impl PaneVisibility {
    /// Whether this node takes part in layout.
    pub fn is_shown(self) -> bool {
        matches!(self, Self::Shown)
    }
}

/// The draggable seam between two adjacent children of a [`Split`].
///
/// Inserted by the builder, never authored by a host: a split with `n` content
/// children has exactly `n - 1` dividers, interleaved.
///
/// It deliberately carries no reference to the nodes it separates. Resize reads
/// its index among its parent's children and takes the neighbours on either
/// side, so reordering children — which is what a future drag-to-rearrange
/// does — needs no divider bookkeeping at all.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Divider;

/// Which way a [`Split`] divides its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    /// Children sit left-to-right; dividers drag horizontally.
    Row,
    /// Children sit top-to-bottom; dividers drag vertically.
    Column,
}

impl Axis {
    /// The bevy_ui flex direction this axis lays children out in.
    pub fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Row => FlexDirection::Row,
            Self::Column => FlexDirection::Column,
        }
    }

    /// The component of a drag delta that moves along this axis.
    pub fn delta_along(self, delta: Vec2) -> f32 {
        match self {
            Self::Row => delta.x,
            Self::Column => delta.y,
        }
    }

    /// The component of a size that runs along this axis.
    pub fn extent_of(self, size: Vec2) -> f32 {
        match self {
            Self::Row => size.x,
            Self::Column => size.y,
        }
    }

    /// The axis at right angles to this one.
    pub fn cross(self) -> Self {
        match self {
            Self::Row => Self::Column,
            Self::Column => Self::Row,
        }
    }

    /// Write `thickness` onto whichever of `node`'s dimensions runs *across*
    /// this axis, leaving the other free to flex.
    ///
    /// This exists because writing the dimension by name is a mistake the
    /// compiler cannot catch and a layout pass reports as nothing at all. A
    /// child declaring `height: Px(24)` inside a row is 24 tall and shares the
    /// width; the identical child inside a *column* is 24 **long** and spans
    /// the full width — measured, not reasoned: three such children in a
    /// 300×300 column come out `300×100` each, not `24×100`.
    ///
    /// Nothing errors, so the only defence is never naming the dimension at a
    /// call site.
    pub fn set_thickness(self, node: &mut Node, thickness: f32) {
        match self {
            // Across a row is vertical; across a column is horizontal.
            Self::Row => node.height = Val::Px(thickness),
            Self::Column => node.width = Val::Px(thickness),
        }
    }

    /// The corner rounding for the item at `index` of `count` along this axis,
    /// where only the strip's two outer ends are rounded.
    ///
    /// A lone item is both first and last and rounds all four corners.
    pub fn end_caps(self, index: usize, count: usize, radius: f32) -> BorderRadius {
        let zero = Val::Px(0.0);
        let end = Val::Px(radius);
        let first = index == 0;
        let last = index + 1 >= count;
        match self {
            // A row rounds its left and right ends.
            Self::Row => BorderRadius {
                top_left: if first { end } else { zero },
                bottom_left: if first { end } else { zero },
                top_right: if last { end } else { zero },
                bottom_right: if last { end } else { zero },
            },
            // A column rounds its top and bottom ends.
            Self::Column => BorderRadius {
                top_left: if first { end } else { zero },
                top_right: if first { end } else { zero },
                bottom_left: if last { end } else { zero },
                bottom_right: if last { end } else { zero },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_size_defaults_to_an_even_flex_weight() {
        assert_eq!(PaneSize::default(), PaneSize::Flex(1.0));
    }

    #[test]
    fn a_fixed_node_has_no_flex_weight() {
        assert_eq!(PaneSize::Flex(2.0).flex(), Some(2.0));
        assert_eq!(PaneSize::Fixed(40.0).flex(), None);
    }

    #[test]
    fn visibility_defaults_to_shown() {
        // A pane declared without an opinion must render. The opposite default
        // would make a fresh layout come up blank.
        assert_eq!(PaneVisibility::default(), PaneVisibility::Shown);
        assert!(PaneVisibility::default().is_shown());
    }

    #[test]
    fn an_axis_reads_the_matching_component_of_a_delta() {
        let delta = Vec2::new(3.0, 7.0);
        assert_eq!(Axis::Row.delta_along(delta), 3.0);
        assert_eq!(Axis::Column.delta_along(delta), 7.0);
    }

    #[test]
    fn thickness_lands_on_the_cross_dimension() {
        // The whole point of `set_thickness`: a row's thickness is its
        // height, a column's is its width, and no call site says which.
        let mut row = Node::default();
        Axis::Row.set_thickness(&mut row, 24.0);
        assert_eq!(row.height, Val::Px(24.0));
        assert_eq!(row.width, Val::Auto, "the length must stay free to flex");

        let mut col = Node::default();
        Axis::Column.set_thickness(&mut col, 24.0);
        assert_eq!(col.width, Val::Px(24.0));
        assert_eq!(col.height, Val::Auto);
    }

    #[test]
    fn end_caps_round_the_two_ends_of_the_axis_they_run_along() {
        let zero = Val::Px(0.0);

        // A row rounds left and right...
        let first = Axis::Row.end_caps(0, 3, 6.0);
        assert_ne!(first.top_left, zero);
        assert_ne!(first.bottom_left, zero);
        assert_eq!(first.top_right, zero);
        let middle = Axis::Row.end_caps(1, 3, 6.0);
        assert_eq!(middle.top_left, zero);
        assert_eq!(middle.top_right, zero);

        // ...a column rounds top and bottom, which is a different pair of
        // corners entirely — not the row's answer with the names swapped.
        let top = Axis::Column.end_caps(0, 3, 6.0);
        assert_ne!(top.top_left, zero);
        assert_ne!(top.top_right, zero, "a column's first item rounds across");
        assert_eq!(top.bottom_left, zero);
        let bottom = Axis::Column.end_caps(2, 3, 6.0);
        assert_ne!(bottom.bottom_left, zero);
        assert_ne!(bottom.bottom_right, zero);
        assert_eq!(bottom.top_left, zero);
    }

    #[test]
    fn a_lone_item_rounds_all_four_corners_on_either_axis() {
        let zero = Val::Px(0.0);
        for axis in [Axis::Row, Axis::Column] {
            let only = axis.end_caps(0, 1, 6.0);
            assert_ne!(only.top_left, zero, "{axis:?}");
            assert_ne!(only.top_right, zero, "{axis:?}");
            assert_ne!(only.bottom_left, zero, "{axis:?}");
            assert_ne!(only.bottom_right, zero, "{axis:?}");
        }
    }

    #[test]
    fn an_empty_strip_does_not_panic_on_its_end_caps() {
        // `count` of 0 has no valid index, but a caller computing
        // `count - 1` would underflow; `end_caps` takes the count itself.
        let _ = Axis::Row.end_caps(0, 0, 6.0);
    }

    #[test]
    fn crossing_an_axis_twice_returns_it() {
        assert_eq!(Axis::Row.cross(), Axis::Column);
        assert_eq!(Axis::Column.cross().cross(), Axis::Column);
    }

    #[test]
    fn a_pane_id_can_be_built_from_anything_stringy() {
        assert_eq!(PaneId::from("browser").as_str(), "browser");
        assert_eq!(PaneId::from(String::from("center")).as_str(), "center");
    }
}
