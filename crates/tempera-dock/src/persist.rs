//! Saving and restoring a layout.
//!
//! The persisted type is [`DockLayout`] itself — there is no separate
//! save-shape to keep in step with the live one, which is what makes a future
//! `Tabs` variant a one-place change rather than three.
//!
//! This crate picks no file format. [`capture`](DockLayout::capture) hands back
//! a `Serialize` value and [`apply`](DockLayout::apply) takes one back; where
//! it goes and in what encoding is the host's business.
//!
//! # Tolerance
//!
//! A saved layout and a running build disagree whenever the app has gained or
//! lost a panel since the file was written. Neither case is an error:
//!
//! - a declared pane the save does not mention is **appended**, with the
//!   size and visibility it was declared with
//! - a saved pane the build does not declare is **kept**
//!
//! Refusing the file would mean one removed panel costs the user their whole
//! arrangement, which is the "an unknown id is ignorable" doctrine
//! `tempera-input` applies to saved keybinds.
//!
//! Keeping unknown panes is the less obvious half, and it is deliberate: the
//! declared tree is a source of *defaults*, not a whitelist. Panes can also be
//! created at runtime — [`DockCommands::split_pane`](crate::DockCommands::split_pane)
//! is exactly that — and those are never in the declared tree. Pruning to what
//! the host declared would silently discard every pane the user made, on every
//! restart.
//!
//! An id that is genuinely retired therefore lingers as an empty pane rather
//! than vanishing. That is visible and fixable; the alternative is not. A host
//! that knows an id is gone for good can drop it with
//! [`DockCommands::remove_pane`](crate::DockCommands::remove_pane), or prune
//! the layout before applying it.

use bevy::prelude::*;

use crate::node::{Axis, PaneId, PaneSize, PaneVisibility};
use crate::tree::{DockLayout, DockTree};

impl DockLayout {
    /// Read the live layout out of the world.
    ///
    /// `None` before the dock has built anything, so a save triggered during
    /// startup writes nothing rather than writing an empty tree over a good
    /// file.
    pub fn capture(world: &World) -> Option<Self> {
        let layout = world.get_resource::<DockLayout>()?;
        // A tree with no panes is a save that would erase a good file.
        layout.pane_ids().next()?;
        Some(layout.clone())
    }

    /// Adopt a saved layout, reconciled against what this build declares.
    ///
    /// The saved tree wins on *shape* — splits, order, sizes, visibility. The
    /// declared tree contributes any pane the save predates. See the module
    /// docs for why unknown saved panes are kept rather than pruned.
    pub fn apply(&self, world: &mut World) {
        let Some(declared) = world.get_resource::<DockLayout>() else {
            warn!("[tempera-dock] apply: no live layout to reconcile against; ignoring");
            return;
        };
        let reconciled = self.reconcile_against(declared);
        if let Err(e) = reconciled.validate() {
            error!(
                "[tempera-dock] saved layout did not reconcile into a valid tree: {e}; keeping the declared layout"
            );
            return;
        }
        world.insert_resource(reconciled);
    }

    /// Merge a saved tree with the panes `declared` adds since it was written.
    pub fn reconcile_against(&self, declared: &DockLayout) -> DockLayout {
        let mut root = self.root.clone();

        // Add panes this build declares that the save predates. Saved panes the
        // build does not declare stay — see the module docs.
        let present: Vec<String> = root.pane_ids().map(|p| p.0.clone()).collect();
        for id in declared.pane_ids() {
            if !present.iter().any(|p| p == id.as_str()) {
                debug!(
                    "[tempera-dock] pane {:?} is new since this layout was saved; appending it",
                    id.as_str()
                );
                root = append_pane(root, declared, id);
            }
        }

        DockLayout {
            version: crate::tree::FORMAT_VERSION,
            root,
        }
    }

    /// Drop every pane whose id fails `keep`, collapsing splits it empties.
    ///
    /// The dock cannot tell a retired panel from a pane the user created, so it
    /// keeps both (see the module docs). A host that *does* know which ids are
    /// gone for good can say so here, before [`apply`](Self::apply).
    ///
    /// Returns the dropped ids. If nothing survives, the layout is left alone —
    /// a tree with no panes is not a layout.
    pub fn retain_panes(&mut self, keep: impl Fn(&str) -> bool) -> Vec<String> {
        let mut dropped = Vec::new();
        match prune(self.root.clone(), &keep, &mut dropped) {
            Some(root) => self.root = root,
            None => {
                warn!("[tempera-dock] retain_panes would empty the layout; keeping it as-is");
                return Vec::new();
            }
        }
        dropped
    }
}

/// Drop every pane failing `keep`, collapsing splits it empties.
///
/// `None` means nothing survived — which the caller must handle, since a tree
/// with no panes is not a layout. Dropped ids are collected so the caller can
/// report them individually rather than as a count.
fn prune(
    tree: DockTree,
    keep: &impl Fn(&str) -> bool,
    dropped: &mut Vec<String>,
) -> Option<DockTree> {
    match tree {
        DockTree::Pane { ref id, .. } => {
            if keep(id.as_str()) {
                Some(tree)
            } else {
                dropped.push(id.0.clone());
                None
            }
        }
        DockTree::Split {
            axis,
            size,
            children,
        } => {
            let kept: Vec<DockTree> = children
                .into_iter()
                .filter_map(|child| prune(child, keep, dropped))
                .collect();
            match kept.len() {
                0 => None,
                // A split reduced to one child is that child, which inherits
                // the split's share — the same rule `remove_pane` follows.
                1 => {
                    let mut survivor = kept.into_iter().next().expect("len checked");
                    match &mut survivor {
                        DockTree::Pane { size: s, .. } | DockTree::Split { size: s, .. } => {
                            *s = size
                        }
                    }
                    Some(survivor)
                }
                _ => Some(DockTree::Split {
                    axis,
                    size,
                    children: kept,
                }),
            }
        }
    }
}

/// Add a pane the saved tree predates, carrying over how it was declared.
///
/// Appended at the root rather than guessed into position: the saved tree is
/// the user's arrangement, and inventing a place inside it would move panes
/// they deliberately put somewhere. Visible at the edge is easy to move; buried
/// in the middle is confusing.
fn append_pane(root: DockTree, declared: &DockLayout, id: &PaneId) -> DockTree {
    let (size, min_size, visibility) = match declared.root.find_pane(id.as_str()) {
        Some(DockTree::Pane {
            size,
            min_size,
            visibility,
            ..
        }) => (*size, *min_size, *visibility),
        _ => (PaneSize::default(), None, PaneVisibility::Shown),
    };
    let fresh = DockTree::Pane {
        id: id.clone(),
        size,
        min_size,
        visibility,
    };

    match root {
        DockTree::Split {
            axis,
            size,
            mut children,
        } => {
            children.push(fresh);
            DockTree::Split {
                axis,
                size,
                children,
            }
        }
        pane => DockTree::split(Axis::Row, [pane, fresh]),
    }
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
    fn an_identical_save_reconciles_to_itself() {
        let declared = row(&["a", "b"]);
        let saved = row(&["a", "b"]);
        assert_eq!(saved.reconcile_against(&declared).root, saved.root);
    }

    #[test]
    fn a_saved_pane_the_build_does_not_declare_is_kept() {
        // The dock cannot tell "panel the app retired" from "pane the user
        // made at runtime" — both are absent from the declared tree. Pruning
        // would silently discard every user-created pane on every restart, so
        // unknown panes stay and the host prunes if it knows better.
        let declared = row(&["a", "b"]);
        let saved = row(&["a", "b", "user_made"]);

        let out = saved.reconcile_against(&declared);
        assert_eq!(ids(&out), ["a", "b", "user_made"]);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn a_host_can_prune_ids_it_knows_are_retired() {
        let mut saved = row(&["a", "b", "retired"]);
        let dropped = saved.retain_panes(|id| id != "retired");

        assert_eq!(dropped, ["retired"]);
        assert_eq!(ids(&saved), ["a", "b"]);
        assert_eq!(saved.validate(), Ok(()));
    }

    #[test]
    fn pruning_everything_leaves_the_layout_alone() {
        // A tree with no panes is not a layout; better to keep a stale one than
        // to hand the builder something it must refuse.
        let mut saved = row(&["a", "b"]);
        let dropped = saved.retain_panes(|_| false);

        assert!(dropped.is_empty());
        assert_eq!(ids(&saved), ["a", "b"]);
    }

    #[test]
    fn pruning_collapses_a_split_it_empties() {
        let mut saved = DockLayout::new(DockTree::split(
            Axis::Column,
            [
                DockTree::pane("keep"),
                DockTree::split(Axis::Row, [DockTree::pane("x"), DockTree::pane("y")]).flex(5.0),
            ],
        ));
        saved.retain_panes(|id| id != "y");

        assert_eq!(ids(&saved), ["keep", "x"]);
        assert_eq!(
            saved.root.find_pane("x").map(DockTree::size),
            Some(PaneSize::Flex(5.0)),
            "the survivor inherits the dissolved split's share"
        );
        assert_eq!(saved.validate(), Ok(()));
    }

    #[test]
    fn a_saved_layout_missing_a_declared_pane_gains_it_back() {
        let declared = row(&["a", "b", "new"]);
        let saved = row(&["a", "b"]);

        let out = saved.reconcile_against(&declared);
        assert_eq!(ids(&out), ["a", "b", "new"]);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn a_new_pane_keeps_the_size_it_was_declared_with() {
        let declared = DockLayout::new(DockTree::split(
            Axis::Row,
            [
                DockTree::pane("a"),
                DockTree::pane("new").fixed(40.0).min_size(10.0),
            ],
        ));
        let saved = DockLayout::new(DockTree::pane("a"));

        let out = saved.reconcile_against(&declared);
        assert_eq!(
            out.root.find_pane("new").map(DockTree::size),
            Some(PaneSize::Fixed(40.0))
        );
    }

    #[test]
    fn a_save_sharing_no_panes_gains_the_declared_ones_and_keeps_its_own() {
        // Nothing in common — an old file, or a different build. Both sets
        // survive; the user sees their arrangement plus whatever is new.
        let declared = row(&["a", "b"]);
        let saved = row(&["x", "y"]);

        let out = saved.reconcile_against(&declared);
        assert_eq!(ids(&out), ["x", "y", "a", "b"]);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn user_sizes_survive_reconciliation() {
        // The whole point of saving: a pane dragged to 3.7 comes back at 3.7,
        // not at whatever the code declares.
        let declared = row(&["a", "b"]);
        let mut saved = row(&["a", "b"]);
        if let DockTree::Split { children, .. } = &mut saved.root {
            children[0] = DockTree::pane("a").flex(3.7);
        }

        let out = saved.reconcile_against(&declared);
        assert_eq!(
            out.root.find_pane("a").map(DockTree::size),
            Some(PaneSize::Flex(3.7))
        );
    }

    #[test]
    fn capture_returns_none_before_anything_is_built() {
        // A save during startup must not write an empty tree over a good file.
        let world = World::new();
        assert!(DockLayout::capture(&world).is_none());
    }

    #[test]
    fn capture_reads_the_live_layout() {
        let mut world = World::new();
        world.insert_resource(row(&["a", "b"]));
        let captured = DockLayout::capture(&world).expect("a built layout");
        assert_eq!(ids(&captured), ["a", "b"]);
    }
}
