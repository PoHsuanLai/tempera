//! The flat list of rows to draw, in order.
//!
//! # Why a flat list and not a spawned scaffold
//!
//! Every tree view that survives a large tree keeps a **flat array of visible
//! rows** and renders from it: VS Code's explorer, Chrome DevTools' DOM tree,
//! IntelliJ's project view, every file manager. Collapsing a folder splices its
//! descendants out of that array. Nobody serious keeps a container element per
//! group — that is the naive nested-list model, and it costs a UI node for
//! every file inside a collapsed `target/`.
//!
//! So this crate computes and **spawns nothing**. A collapsed subtree is
//! *absent from the list*, not present and hidden, and the algorithm is a pure
//! function that needs no `App` to test.
//!
//! The honest cost: toggling a group re-emits the list, so rows respawn where a
//! container model would only have flipped a `Display`. The browser this came
//! from called out that "no respawn, no flash" property deliberately — but it
//! held for exactly one of five triggers, since a catalog change, a query
//! keystroke, a section toggle or a tint change already respawned everything.
//! A host that needs it back can diff this list against what it has spawned,
//! which the flat model permits and the container model forecloses.
//!
//! # Malformed hierarchy is reported, not fatal
//!
//! A declared dock layout can be validated up front. This cannot: the hierarchy
//! comes from a filesystem walk or a plugin scan, so a duplicate group id or a
//! parent cycle is only detectable while walking it. Each is warned about once
//! and the offending item is dropped — never a panic, and never taking a
//! sibling down with it.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::filter::{FilterNode, surviving};
use crate::item::GroupId;
use crate::state::TreeState;

/// One item, as [`visible_rows`] needs to see it.
///
/// A host builds these from its own query. Keeping the input a borrowed slice
/// rather than a `World` is what makes the whole computation testable without
/// an `App`.
#[derive(Debug, Clone, Copy)]
pub struct TreeNode<'a> {
    /// The host's entity, handed back untouched on [`VisibleRow::item`].
    pub entity: Entity,
    /// Sort and match key.
    pub name: &'a str,
    /// `Some` ⇒ this row is a group and can be collapsed.
    pub group: Option<&'a GroupId>,
    /// The group this row sits in, `None` at a root.
    pub parent: Option<&'a GroupId>,
    /// Whether this group starts open when the user has no opinion.
    /// Meaningless on a leaf.
    pub default_open: bool,
}

/// One visible row, in final render order.
///
/// Pure output: hierarchy resolved, filter applied, collapsed subtrees omitted,
/// depth derived.
///
/// # `item` is a hint, not a guarantee
///
/// Between computing this list and rendering from it, a rescan can despawn the
/// entity — extension catalogs replace their items wholesale. Read it with
/// `world.get_entity(row.item).ok()` and skip a miss; never index it blind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleRow {
    /// The host's entity. See the type docs before dereferencing it.
    pub item: Entity,
    /// 0 at a root, +1 per level.
    pub depth: u16,
    /// Whether this row is expandable.
    pub is_group: bool,
    /// Whether it is currently open. Meaningless when `is_group` is false.
    pub expanded: bool,
}

/// The rows to draw, in order.
///
/// Groups sort before leaves, then alphabetically case-insensitively — stable
/// ordering, so a rescan does not shuffle the list.
///
/// A live `query` **flattens without mutating state**: every group renders open
/// for the duration, and clearing the query restores exactly the expansion the
/// user had. Persisting a search's incidental expansion would silently rewrite
/// a user's layout as a side effect of typing.
pub fn visible_rows(nodes: &[TreeNode<'_>], state: &TreeState, query: &str) -> Vec<VisibleRow> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let kept = {
        let filter_nodes: Vec<FilterNode<'_>> = nodes
            .iter()
            .map(|n| FilterNode {
                name: n.name,
                group: n.group,
                parent: n.parent,
            })
            .collect();
        surviving(&filter_nodes, query)
    };

    // Which node owns each group id. A second node claiming an id is dropped:
    // the id is how a parent link and the saved state both name a group, so two
    // claimants make "open src" ambiguous.
    let mut node_of_group: HashMap<&GroupId, usize> = HashMap::new();
    let mut duplicates: HashSet<usize> = HashSet::new();
    for (i, node) in nodes.iter().enumerate() {
        let Some(group) = node.group else { continue };
        if let Some(&first) = node_of_group.get(group) {
            warn!(
                "[tempera-tree] two items claim the group id {:?} ({:?} and {:?}); \
                 keeping the first and dropping the second",
                group.as_str(),
                nodes[first].entity,
                node.entity,
            );
            duplicates.insert(i);
        } else {
            node_of_group.insert(group, i);
        }
    }

    // Bucket by parent. An item naming a parent that does not exist is dropped
    // — rendering it at a root would move it somewhere the user did not put it
    // — but its siblings are unaffected.
    let mut roots: Vec<usize> = Vec::new();
    let mut children_of: HashMap<&GroupId, Vec<usize>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        if duplicates.contains(&i) || !kept.contains(&i) {
            continue;
        }
        match node.parent {
            None => roots.push(i),
            Some(parent) if node_of_group.contains_key(parent) => {
                children_of.entry(parent).or_default().push(i);
            }
            Some(parent) => {
                warn!(
                    "[tempera-tree] {:?} names parent group {:?}, which no item declares; \
                     dropping the row",
                    node.entity,
                    parent.as_str(),
                );
            }
        }
    }

    let order = |a: &usize, b: &usize| {
        let (a, b) = (&nodes[*a], &nodes[*b]);
        b.group
            .is_some()
            .cmp(&a.group.is_some())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    };
    roots.sort_by(order);
    for siblings in children_of.values_mut() {
        siblings.sort_by(order);
    }

    let searching = !query.trim().is_empty();
    let mut out = Vec::with_capacity(kept.len());
    // Guards a `ParentGroup` cycle, which a scanner can emit. Without it the
    // walk recurses until the stack goes.
    let mut open_path: HashSet<&GroupId> = HashSet::new();
    let mut stack: Vec<(usize, u16)> = roots.into_iter().rev().map(|i| (i, 0)).collect();
    // Groups whose subtree has been pushed, so we can pop them off `open_path`.
    let mut pending_close: Vec<(usize, &GroupId)> = Vec::new();

    while let Some((index, depth)) = stack.pop() {
        while let Some(&(at, group)) = pending_close.last() {
            if at == stack.len() {
                open_path.remove(group);
                pending_close.pop();
            } else {
                break;
            }
        }

        let node = &nodes[index];
        let expanded = match node.group {
            // A search flattens: every group reads open, and `state` is untouched.
            Some(group) => searching || state.is_open(group, node.default_open),
            None => false,
        };

        out.push(VisibleRow {
            item: node.entity,
            depth,
            is_group: node.group.is_some(),
            expanded,
        });

        let Some(group) = node.group else { continue };
        if !expanded {
            continue;
        }
        if !open_path.insert(group) {
            warn!(
                "[tempera-tree] group {:?} is its own ancestor; stopping the walk there",
                group.as_str(),
            );
            continue;
        }
        pending_close.push((stack.len(), group));
        for &child in children_of.get(group).into_iter().flatten().rev() {
            stack.push((child, depth + 1));
        }
    }

    out
}
