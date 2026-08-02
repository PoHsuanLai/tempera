//! The declared tree: what a host asks for, and what goes to disk.
//!
//! Plain data — no `Entity`, no `World`. That is what lets one type serve three
//! jobs that would otherwise drift apart: a host declares a [`DockLayout`], a
//! saved layout deserializes into the same type, and a runtime rearrangement
//! edits it in place. One shape, one set of bugs.
//!
//! ```
//! use tempera_dock::{Axis, DockLayout, DockTree};
//!
//! let layout = DockLayout::new(DockTree::split(
//!     Axis::Row,
//!     [
//!         DockTree::pane("browser").flex(1.0),
//!         DockTree::pane("center").flex(6.0),
//!     ],
//! ));
//! assert!(layout.validate().is_ok());
//! assert_eq!(layout.pane_ids().count(), 2);
//! ```

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::node::{Axis, PaneId, PaneSize, PaneVisibility};

/// The layout a host wants, as a resource.
///
/// Insert it before the dock's build runs and the tree is materialized into
/// entities. Replace it later and the tree is reconciled to match.
#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockLayout {
    /// Schema version of the persisted form.
    ///
    /// Present from the first release so a later format change has somewhere
    /// to branch. A file without it parses as version 1 rather than failing —
    /// this is per-machine layout state, and refusing to open a project over a
    /// dock layout would be a poor trade.
    #[serde(default = "default_version")]
    pub version: u32,
    /// The root node.
    pub root: DockTree,
}

/// The current persisted schema version.
pub const FORMAT_VERSION: u32 = 1;

fn default_version() -> u32 {
    FORMAT_VERSION
}

impl DockLayout {
    /// A layout with `root` at the top.
    pub fn new(root: DockTree) -> Self {
        Self {
            version: FORMAT_VERSION,
            root,
        }
    }

    /// Every pane id in the tree, depth-first, left to right.
    pub fn pane_ids(&self) -> impl Iterator<Item = &PaneId> {
        self.root.pane_ids()
    }

    /// Check the tree is buildable.
    ///
    /// See [`TreeError`] for what can go wrong and why each case is fatal
    /// rather than repairable.
    pub fn validate(&self) -> Result<(), TreeError> {
        let mut seen = HashSet::new();
        self.root.validate(&mut seen)
    }
}

/// One node of a declared tree.
///
/// Recursive on disk from the first version, which is what makes a later
/// `Tabs` variant additive: `Pane` and `Split` keep their shape, so layouts
/// saved before it existed still parse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DockTree {
    /// A leaf that content parents into.
    Pane {
        /// Stable identity, and the key content looks this pane up by.
        id: PaneId,
        /// Share of the parent split.
        #[serde(default)]
        size: PaneSize,
        /// Floor below which a drag will not shrink this pane.
        #[serde(default)]
        min_size: Option<f32>,
        /// Whether the pane takes part in layout.
        #[serde(default)]
        visibility: PaneVisibility,
    },
    /// A branch dividing its children along one axis.
    Split {
        /// Which way children are laid out.
        axis: Axis,
        /// Share of the *parent* split.
        #[serde(default)]
        size: PaneSize,
        /// Child nodes, in layout order. Dividers are inserted between them by
        /// the builder and are not represented here.
        children: Vec<DockTree>,
    },
}

impl DockTree {
    /// A leaf with the given id, a flex weight of 1, visible, no minimum.
    pub fn pane(id: impl Into<PaneId>) -> Self {
        Self::Pane {
            id: id.into(),
            size: PaneSize::default(),
            min_size: None,
            visibility: PaneVisibility::Shown,
        }
    }

    /// A branch over `children`, laid out along `axis`.
    pub fn split(axis: Axis, children: impl IntoIterator<Item = DockTree>) -> Self {
        Self::Split {
            axis,
            size: PaneSize::default(),
            children: children.into_iter().collect(),
        }
    }

    /// Claim `weight` share of the parent split, relative to siblings.
    pub fn flex(mut self, weight: f32) -> Self {
        *self.size_mut() = PaneSize::Flex(weight);
        self
    }

    /// Claim exactly `px` logical pixels, unaffected by sibling drags.
    pub fn fixed(mut self, px: f32) -> Self {
        *self.size_mut() = PaneSize::Fixed(px);
        self
    }

    /// Refuse to shrink below `px` logical pixels.
    ///
    /// Only meaningful on a pane; a split sizes from its children.
    pub fn min_size(mut self, px: f32) -> Self {
        if let Self::Pane { min_size, .. } = &mut self {
            *min_size = Some(px);
        }
        self
    }

    /// Start hidden.
    pub fn hidden(mut self) -> Self {
        if let Self::Pane { visibility, .. } = &mut self {
            *visibility = PaneVisibility::Hidden;
        }
        self
    }

    /// This node's share of its parent split.
    pub fn size(&self) -> PaneSize {
        match self {
            Self::Pane { size, .. } | Self::Split { size, .. } => *size,
        }
    }

    fn size_mut(&mut self) -> &mut PaneSize {
        match self {
            Self::Pane { size, .. } | Self::Split { size, .. } => size,
        }
    }

    /// Every pane id at or below this node, depth-first, left to right.
    pub fn pane_ids(&self) -> Box<dyn Iterator<Item = &PaneId> + '_> {
        match self {
            Self::Pane { id, .. } => Box::new(std::iter::once(id)),
            Self::Split { children, .. } => {
                Box::new(children.iter().flat_map(|child| child.pane_ids()))
            }
        }
    }

    /// Find a pane by id, anywhere at or below this node.
    pub fn find_pane(&self, id: &str) -> Option<&DockTree> {
        match self {
            Self::Pane { id: pane_id, .. } if pane_id.as_str() == id => Some(self),
            Self::Pane { .. } => None,
            Self::Split { children, .. } => children.iter().find_map(|c| c.find_pane(id)),
        }
    }

    fn validate(&self, seen: &mut HashSet<String>) -> Result<(), TreeError> {
        match self {
            Self::Pane { id, .. } => {
                if !seen.insert(id.0.clone()) {
                    return Err(TreeError::DuplicatePaneId(id.0.clone()));
                }
                Ok(())
            }
            Self::Split { children, .. } => {
                if children.len() < 2 {
                    return Err(TreeError::DegenerateSplit(children.len()));
                }
                for child in children {
                    child.validate(seen)?;
                }
                Ok(())
            }
        }
    }
}

/// Why a declared tree cannot be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    /// Two panes share an id.
    ///
    /// Fatal rather than repairable: content finds its pane *by* id, so a
    /// duplicate means some panel silently parents into the wrong rectangle —
    /// a bug that presents as a layout glitch far from its cause.
    DuplicatePaneId(String),
    /// A split with fewer than two children.
    ///
    /// A split of one is that child wearing a costume: it has no divider, no
    /// sibling to resize against, and would make `remove_pane`'s
    /// collapse-the-parent rule ambiguous. The builder never produces one, so
    /// a host should not declare one either.
    DegenerateSplit(usize),
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePaneId(id) => {
                write!(f, "two panes share the id {id:?}; pane ids must be unique")
            }
            Self::DegenerateSplit(n) => {
                write!(f, "a split needs at least 2 children, got {n}")
            }
        }
    }
}

impl std::error::Error for TreeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_pane_row() -> DockLayout {
        DockLayout::new(DockTree::split(
            Axis::Row,
            [
                DockTree::pane("browser").flex(1.0),
                DockTree::pane("center").flex(6.0),
            ],
        ))
    }

    #[test]
    fn a_declared_pane_defaults_to_an_even_flex_weight() {
        assert_eq!(DockTree::pane("a").size(), PaneSize::Flex(1.0));
    }

    #[test]
    fn builders_set_size_min_and_visibility() {
        let pane = DockTree::pane("a").fixed(40.0);
        assert_eq!(pane.size(), PaneSize::Fixed(40.0));

        let pane = DockTree::pane("b").flex(3.0).min_size(120.0).hidden();
        assert_eq!(pane.size(), PaneSize::Flex(3.0));
        match pane {
            DockTree::Pane {
                min_size,
                visibility,
                ..
            } => {
                assert_eq!(min_size, Some(120.0));
                assert_eq!(visibility, PaneVisibility::Hidden);
            }
            _ => panic!("expected a pane"),
        }
    }

    #[test]
    fn pane_ids_walk_depth_first_left_to_right() {
        let layout = DockLayout::new(DockTree::split(
            Axis::Column,
            [
                DockTree::pane("top"),
                DockTree::split(Axis::Row, [DockTree::pane("left"), DockTree::pane("right")]),
            ],
        ));
        let ids: Vec<_> = layout.pane_ids().map(|p| p.as_str()).collect();
        assert_eq!(ids, ["top", "left", "right"]);
    }

    #[test]
    fn a_valid_tree_validates() {
        assert_eq!(two_pane_row().validate(), Ok(()));
    }

    #[test]
    fn duplicate_pane_ids_are_rejected() {
        // Content looks its pane up by id. A duplicate means some panel
        // parents into the wrong rectangle, so this must fail loudly at
        // declaration rather than surface as a layout glitch later.
        let layout = DockLayout::new(DockTree::split(
            Axis::Row,
            [DockTree::pane("dup"), DockTree::pane("dup")],
        ));
        assert_eq!(
            layout.validate(),
            Err(TreeError::DuplicatePaneId("dup".into()))
        );
    }

    #[test]
    fn duplicate_ids_are_caught_across_nesting_levels() {
        let layout = DockLayout::new(DockTree::split(
            Axis::Column,
            [
                DockTree::pane("same"),
                DockTree::split(Axis::Row, [DockTree::pane("other"), DockTree::pane("same")]),
            ],
        ));
        assert!(matches!(
            layout.validate(),
            Err(TreeError::DuplicatePaneId(_))
        ));
    }

    #[test]
    fn a_split_with_one_child_is_rejected() {
        let layout = DockLayout::new(DockTree::split(Axis::Row, [DockTree::pane("only")]));
        assert_eq!(layout.validate(), Err(TreeError::DegenerateSplit(1)));
    }

    #[test]
    fn an_empty_split_is_rejected() {
        let layout = DockLayout::new(DockTree::split(Axis::Row, []));
        assert_eq!(layout.validate(), Err(TreeError::DegenerateSplit(0)));
    }

    #[test]
    fn a_lone_pane_is_a_valid_root() {
        // Not a degenerate split — a single-pane app is a real shape.
        let layout = DockLayout::new(DockTree::pane("only"));
        assert_eq!(layout.validate(), Ok(()));
    }

    #[test]
    fn find_pane_reaches_into_nested_splits() {
        let layout = DockLayout::new(DockTree::split(
            Axis::Column,
            [
                DockTree::pane("top"),
                DockTree::split(
                    Axis::Row,
                    [DockTree::pane("left"), DockTree::pane("right").flex(9.0)],
                ),
            ],
        ));
        assert_eq!(
            layout.root.find_pane("right").map(DockTree::size),
            Some(PaneSize::Flex(9.0))
        );
        assert_eq!(layout.root.find_pane("nope"), None);
    }
}
