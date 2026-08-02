//! Changing the tree at runtime.
//!
//! Every mutation edits the declared [`DockLayout`] and asks for a rebuild,
//! rather than surgically editing entities. That is what keeps "declared",
//! "restored from disk" and "rearranged by the user" on one code path: a bug
//! in the tree surgery shows up in all three, instead of only in the one
//! nobody tested.
//!
//! This is also where a future drag-to-rearrange lands — [`DockCommands::move_pane`]
//! is the whole of the tree surgery it needs, written and tested before any
//! drag code exists.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::build::RebuildRequested;
use crate::node::{Axis, PaneId, PaneSize};
use crate::tree::{DockLayout, DockTree};

/// Which side of a target a pane goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Left of, or above, the target.
    Before,
    /// Right of, or below, the target.
    After,
}

/// Structural edits to the live tree.
#[derive(SystemParam)]
pub struct DockCommands<'w> {
    layout: ResMut<'w, DockLayout>,
    rebuild: ResMut<'w, RebuildRequested>,
}

impl DockCommands<'_> {
    /// Split `target` along `axis`, putting a new pane `new_pane` on `side`.
    ///
    /// No-op with a warning if `target` is not a live pane, or if `new_pane`
    /// already exists — a duplicate id would make one of the two unfindable.
    pub fn split_pane(
        &mut self,
        target: &str,
        axis: Axis,
        new_pane: impl Into<PaneId>,
        side: Side,
    ) {
        let new_pane = new_pane.into();
        if self.layout.root.find_pane(new_pane.as_str()).is_some() {
            warn!(
                "[tempera-dock] split_pane: {:?} already exists; ignoring",
                new_pane.as_str()
            );
            return;
        }
        if self.layout.root.find_pane(target).is_none() {
            warn!("[tempera-dock] split_pane: no pane {target:?}; ignoring");
            return;
        }

        let replaced = replace_pane(&mut self.layout.root, target, |pane| {
            let size = pane.size();
            let fresh = DockTree::pane(new_pane.clone());
            // The new split inherits the target's share, and the two panes
            // divide it evenly — the target keeps the space it had.
            let children = match side {
                Side::Before => vec![fresh, pane.flex(1.0)],
                Side::After => vec![pane.flex(1.0), fresh],
            };
            let mut split = DockTree::split(axis, children);
            *split_size_mut(&mut split) = size;
            split
        });
        if replaced {
            self.request_rebuild();
        }
    }

    /// Remove `target` from the tree.
    ///
    /// A split left with a single child is replaced by that child, which
    /// inherits the split's share — otherwise the tree accumulates splits that
    /// divide nothing, and `validate` would reject its own output.
    ///
    /// No-op with a warning if `target` is not a live pane. Refuses to remove
    /// the last pane: a tree with no panes has nothing to show.
    pub fn remove_pane(&mut self, target: &str) {
        if self.layout.root.find_pane(target).is_none() {
            warn!("[tempera-dock] remove_pane: no pane {target:?}; ignoring");
            return;
        }
        if self.layout.pane_ids().count() <= 1 {
            warn!("[tempera-dock] remove_pane: {target:?} is the last pane; ignoring");
            return;
        }
        if remove_pane_from(&mut self.layout.root, target) {
            self.request_rebuild();
        }
    }

    /// Move `target` next to `into`, on `side`.
    ///
    /// Remove-then-insert, so it inherits both the collapse rule above and the
    /// sizing rule from [`split_pane`](Self::split_pane).
    pub fn move_pane(&mut self, target: &str, into: &str, axis: Axis, side: Side) {
        if target == into {
            return;
        }
        let Some(moved) = self.layout.root.find_pane(target).cloned() else {
            warn!("[tempera-dock] move_pane: no pane {target:?}; ignoring");
            return;
        };
        if self.layout.root.find_pane(into).is_none() {
            warn!("[tempera-dock] move_pane: no pane {into:?}; ignoring");
            return;
        }
        if !remove_pane_from(&mut self.layout.root, target) {
            return;
        }
        let replaced = replace_pane(&mut self.layout.root, into, |pane| {
            let size = pane.size();
            let children = match side {
                Side::Before => vec![moved.clone().flex(1.0), pane.flex(1.0)],
                Side::After => vec![pane.flex(1.0), moved.clone().flex(1.0)],
            };
            let mut split = DockTree::split(axis, children);
            *split_size_mut(&mut split) = size;
            split
        });
        if replaced {
            self.request_rebuild();
        }
    }

    fn request_rebuild(&mut self) {
        self.rebuild.0 = true;
    }
}

fn split_size_mut(tree: &mut DockTree) -> &mut PaneSize {
    match tree {
        DockTree::Pane { size, .. } | DockTree::Split { size, .. } => size,
    }
}

/// Swap the pane named `id` for whatever `f` builds from it.
fn replace_pane(
    tree: &mut DockTree,
    id: &str,
    f: impl FnOnce(DockTree) -> DockTree + Clone,
) -> bool {
    match tree {
        DockTree::Pane { id: pane_id, .. } if pane_id.as_str() == id => {
            // `f` consumes the old node, so swap a placeholder in to take
            // ownership without cloning the subtree.
            let placeholder = DockTree::pane(pane_id.clone());
            let old = std::mem::replace(tree, placeholder);
            *tree = f(old);
            true
        }
        DockTree::Pane { .. } => false,
        DockTree::Split { children, .. } => children
            .iter_mut()
            .any(|child| replace_pane(child, id, f.clone())),
    }
}

/// Drop the pane named `id`, collapsing any split it leaves with one child.
fn remove_pane_from(tree: &mut DockTree, id: &str) -> bool {
    let DockTree::Split { children, .. } = tree else {
        return false;
    };

    if let Some(index) = children
        .iter()
        .position(|c| matches!(c, DockTree::Pane { id: pane_id, .. } if pane_id.as_str() == id))
    {
        children.remove(index);
        collapse_if_single(tree);
        return true;
    }

    for child in children.iter_mut() {
        if remove_pane_from(child, id) {
            collapse_if_single(tree);
            return true;
        }
    }
    false
}

/// A split with one child *is* that child; the survivor inherits the share.
fn collapse_if_single(tree: &mut DockTree) {
    let DockTree::Split { children, size, .. } = tree else {
        return;
    };
    if children.len() != 1 {
        return;
    }
    let inherited = *size;
    let mut survivor = children.remove(0);
    *split_size_mut(&mut survivor) = inherited;
    *tree = survivor;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(ids: &[&str]) -> DockLayout {
        DockLayout::new(DockTree::split(
            Axis::Row,
            ids.iter().map(|id| DockTree::pane(*id)),
        ))
    }

    fn ids(layout: &DockLayout) -> Vec<String> {
        layout.pane_ids().map(|p| p.0.clone()).collect()
    }

    #[test]
    fn removing_the_second_of_two_panes_collapses_the_split() {
        // The split has nothing left to divide, so it must dissolve — a split
        // of one would fail the crate's own validation.
        let mut layout = row(&["a", "b"]);
        assert!(remove_pane_from(&mut layout.root, "b"));

        assert!(matches!(layout.root, DockTree::Pane { .. }));
        assert_eq!(ids(&layout), ["a"]);
        assert_eq!(layout.validate(), Ok(()));
    }

    #[test]
    fn collapsing_promotes_the_survivors_size() {
        // The split held a share of *its* parent; dissolving it must not hand
        // that share back, or the sibling silently grows.
        let mut layout = DockLayout::new(DockTree::split(
            Axis::Column,
            [
                DockTree::pane("top"),
                DockTree::split(Axis::Row, [DockTree::pane("a"), DockTree::pane("b")]).flex(5.0),
            ],
        ));
        assert!(remove_pane_from(&mut layout.root, "b"));

        assert_eq!(
            layout.root.find_pane("a").map(DockTree::size),
            Some(PaneSize::Flex(5.0)),
            "the survivor inherits the dissolved split's share"
        );
    }

    #[test]
    fn removing_one_of_three_leaves_the_split_intact() {
        let mut layout = row(&["a", "b", "c"]);
        assert!(remove_pane_from(&mut layout.root, "b"));

        assert!(matches!(layout.root, DockTree::Split { .. }));
        assert_eq!(ids(&layout), ["a", "c"]);
        assert_eq!(layout.validate(), Ok(()));
    }

    #[test]
    fn removing_an_unknown_id_changes_nothing() {
        let mut layout = row(&["a", "b"]);
        assert!(!remove_pane_from(&mut layout.root, "nope"));
        assert_eq!(ids(&layout), ["a", "b"]);
    }

    #[test]
    fn nested_removal_collapses_every_level_it_empties() {
        // Two nested splits both reduced to one child; both must dissolve, or
        // the tree keeps a chain of splits dividing nothing.
        let mut layout = DockLayout::new(DockTree::split(
            Axis::Column,
            [
                DockTree::pane("keep"),
                DockTree::split(
                    Axis::Row,
                    [
                        DockTree::pane("x"),
                        DockTree::split(Axis::Column, [DockTree::pane("y"), DockTree::pane("z")]),
                    ],
                ),
            ],
        ));
        assert!(remove_pane_from(&mut layout.root, "y"));
        assert!(remove_pane_from(&mut layout.root, "z"));

        assert_eq!(ids(&layout), ["keep", "x"]);
        assert_eq!(layout.validate(), Ok(()));
    }

    #[test]
    fn replace_pane_swaps_a_leaf_for_a_split() {
        let mut layout = row(&["a", "b"]);
        let replaced = replace_pane(&mut layout.root, "b", |pane| {
            DockTree::split(Axis::Column, [pane, DockTree::pane("new")])
        });

        assert!(replaced);
        assert_eq!(ids(&layout), ["a", "b", "new"]);
        assert_eq!(layout.validate(), Ok(()));
    }

    #[test]
    fn replace_pane_reports_a_miss() {
        let mut layout = row(&["a", "b"]);
        assert!(!replace_pane(&mut layout.root, "nope", |pane| pane));
        assert_eq!(ids(&layout), ["a", "b"]);
    }
}
