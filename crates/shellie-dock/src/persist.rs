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
//! - a saved pane the build no longer declares is **dropped**, with a warning
//! - a declared pane the save does not mention is **appended**, with defaults
//!
//! The alternative — refusing the file — means one removed panel makes a user
//! lose their whole layout. This mirrors the "an unknown id is ignorable"
//! doctrine `shellie-input` applies to saved keybinds.

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
    /// The saved tree supplies the *shape* — splits, order, sizes, visibility.
    /// The declared tree supplies the *set of panes that exist*. See the module
    /// docs for how the two are reconciled when they disagree.
    pub fn apply(&self, world: &mut World) {
        let Some(declared) = world.get_resource::<DockLayout>() else {
            warn!("[shellie-dock] apply: no live layout to reconcile against; ignoring");
            return;
        };
        let reconciled = self.reconcile_against(declared);
        if let Err(e) = reconciled.validate() {
            error!(
                "[shellie-dock] saved layout did not reconcile into a valid tree: {e}; keeping the declared layout"
            );
            return;
        }
        world.insert_resource(reconciled);
    }

    /// Merge a saved tree with the set of panes `declared` says exist.
    pub fn reconcile_against(&self, declared: &DockLayout) -> DockLayout {
        let known: Vec<&PaneId> = declared.pane_ids().collect();

        // Drop saved panes this build no longer has.
        let mut dropped = Vec::new();
        let pruned = prune(
            self.root.clone(),
            &|id| known.iter().any(|k| k.as_str() == id),
            &mut dropped,
        );
        for id in &dropped {
            warn!("[shellie-dock] saved layout names unknown pane {id:?}; dropping it");
        }

        let mut root = pruned.unwrap_or_else(|| {
            warn!("[shellie-dock] saved layout had no recognisable panes; using the declared tree");
            declared.root.clone()
        });

        // Append panes this build declares that the save predates.
        let present: Vec<String> = root.pane_ids().map(|p| p.0.clone()).collect();
        for id in known {
            if !present.iter().any(|p| p == id.as_str()) {
                warn!(
                    "[shellie-dock] pane {:?} is new since this layout was saved; appending it",
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
    fn a_saved_layout_naming_an_unknown_pane_drops_it() {
        // The app removed a panel since this file was written. Losing the pane
        // is right; losing the whole layout is not.
        let declared = row(&["a", "b"]);
        let saved = row(&["a", "b", "removed"]);

        let out = saved.reconcile_against(&declared);
        assert_eq!(ids(&out), ["a", "b"]);
        assert_eq!(out.validate(), Ok(()));
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
    fn dropping_a_pane_collapses_the_split_it_empties() {
        let declared = row(&["keep"]);
        let saved = DockLayout::new(DockTree::split(
            Axis::Column,
            [
                DockTree::pane("keep"),
                DockTree::split(
                    Axis::Row,
                    [DockTree::pane("gone_a"), DockTree::pane("gone_b")],
                ),
            ],
        ));

        let out = saved.reconcile_against(&declared);
        assert_eq!(ids(&out), ["keep"]);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn a_save_sharing_no_panes_falls_back_to_the_declared_tree() {
        let declared = row(&["a", "b"]);
        let saved = row(&["x", "y"]);

        let out = saved.reconcile_against(&declared);
        assert_eq!(ids(&out), ["a", "b"]);
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
