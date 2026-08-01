//! Which items survive a query.
//!
//! One predicate, in one place. The browser this was extracted from had two
//! copies with the same intent — one deciding which rows to draw, one counting
//! matches for a header badge — which is two chances to drift.
//!
//! # Ancestor-preserving
//!
//! A node survives if **it or any descendant matches**. That is the whole
//! difference between a tree filter and a list filter, and getting it wrong is
//! the defect this replaces: filtering each item independently drops a folder
//! whose own name misses the query, and the matching file inside it goes with
//! it. Searching for `main.rs` returned nothing because `src` is not `main.rs`.
//!
//! A matching *group* keeps its whole subtree, matching or not. Searching for a
//! folder shows what is in it, which is what every file manager does.
//!
//! Nothing here touches the `World`.

use std::collections::{HashMap, HashSet};

use crate::item::GroupId;

/// One node, as the filter sees it.
///
/// Deliberately not an `Entity`: the filter is a pure function over names and
/// parent links, and keeping it that way is what makes it testable without an
/// `App`.
#[derive(Debug, Clone, Copy)]
pub struct FilterNode<'a> {
    /// What a query matches against.
    pub name: &'a str,
    /// `Some` if this node is a group.
    pub group: Option<&'a GroupId>,
    /// The group this node sits in, `None` at a root.
    pub parent: Option<&'a GroupId>,
}

/// Does `name` match `query`?
///
/// Case-insensitive substring. An empty query matches everything, so callers do
/// not special-case it.
pub fn matches(name: &str, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&q.to_lowercase())
}

/// Indices into `nodes` that survive `query`, ancestor-preserving.
///
/// Returns every index when the query is blank, without doing the work.
///
/// Three passes, all linear: index the parent links, propagate a direct match
/// up to the root, then push a matching group's survival down over its subtree.
pub fn surviving(nodes: &[FilterNode<'_>], query: &str) -> HashSet<usize> {
    if query.trim().is_empty() {
        return (0..nodes.len()).collect();
    }

    // Where each group's own node lives, and who sits under each group.
    let mut node_of_group: HashMap<&GroupId, usize> = HashMap::new();
    let mut children_of: HashMap<&GroupId, Vec<usize>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        if let Some(group) = node.group {
            node_of_group.insert(group, i);
        }
        if let Some(parent) = node.parent {
            children_of.entry(parent).or_default().push(i);
        }
    }

    let mut survives: HashSet<usize> = HashSet::new();

    // A direct match keeps itself and every ancestor, so the path to it stays
    // reachable. The `seen` guard is what makes a `ParentGroup` cycle — which a
    // scanner can produce and this crate must not hang on — terminate.
    for (i, node) in nodes.iter().enumerate() {
        if !matches(node.name, query) {
            continue;
        }
        survives.insert(i);

        let mut seen: HashSet<&GroupId> = HashSet::new();
        let mut cursor = node.parent;
        while let Some(group) = cursor {
            if !seen.insert(group) {
                break;
            }
            let Some(&idx) = node_of_group.get(group) else {
                break;
            };
            survives.insert(idx);
            cursor = nodes[idx].parent;
        }
    }

    // A group that matched on its own name keeps everything inside it.
    let roots: Vec<&GroupId> = nodes
        .iter()
        .enumerate()
        .filter(|(i, n)| survives.contains(i) && matches(n.name, query))
        .filter_map(|(_, n)| n.group)
        .collect();

    let mut stack = roots;
    let mut expanded: HashSet<&GroupId> = HashSet::new();
    while let Some(group) = stack.pop() {
        if !expanded.insert(group) {
            continue;
        }
        for &child in children_of.get(group).into_iter().flatten() {
            survives.insert(child);
            if let Some(g) = nodes[child].group {
                stack.push(g);
            }
        }
    }

    survives
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gid(s: &str) -> GroupId {
        GroupId(s.to_string())
    }

    /// `src/ { main.rs, widgets/ { button.rs } }`, `assets/ { kick.wav }`.
    fn fixture() -> (
        Vec<GroupId>,
        Vec<(&'static str, Option<usize>, Option<usize>)>,
    ) {
        let groups = vec![gid("src"), gid("widgets"), gid("assets")];
        // (name, own group index, parent group index)
        let rows = vec![
            ("src", Some(0), None),
            ("main.rs", None, Some(0)),
            ("widgets", Some(1), Some(0)),
            ("button.rs", None, Some(1)),
            ("assets", Some(2), None),
            ("kick.wav", None, Some(2)),
        ];
        (groups, rows)
    }

    fn build<'a>(
        groups: &'a [GroupId],
        rows: &'a [(&'static str, Option<usize>, Option<usize>)],
    ) -> Vec<FilterNode<'a>> {
        rows.iter()
            .map(|(name, own, parent)| FilterNode {
                name,
                group: own.map(|i| &groups[i]),
                parent: parent.map(|i| &groups[i]),
            })
            .collect()
    }

    fn names(nodes: &[FilterNode<'_>], set: &HashSet<usize>) -> Vec<String> {
        let mut out: Vec<String> = set.iter().map(|&i| nodes[i].name.to_string()).collect();
        out.sort();
        out
    }

    #[test]
    fn a_matching_leaf_keeps_its_ancestors() {
        // The regression this crate exists to fix: filtering item-by-item drops
        // `src` (its name is not "main.rs") and the match goes down with it.
        let (groups, rows) = fixture();
        let nodes = build(&groups, &rows);

        let kept = surviving(&nodes, "main.rs");
        assert_eq!(names(&nodes, &kept), ["main.rs", "src"]);
    }

    #[test]
    fn ancestors_are_kept_all_the_way_up() {
        let (groups, rows) = fixture();
        let nodes = build(&groups, &rows);

        let kept = surviving(&nodes, "button");
        assert_eq!(names(&nodes, &kept), ["button.rs", "src", "widgets"]);
    }

    #[test]
    fn a_matching_group_keeps_its_whole_subtree() {
        // Searching a folder shows what is in it, including the nested folder
        // and its contents.
        let (groups, rows) = fixture();
        let nodes = build(&groups, &rows);

        let kept = surviving(&nodes, "src");
        assert_eq!(
            names(&nodes, &kept),
            ["button.rs", "main.rs", "src", "widgets"]
        );
    }

    #[test]
    fn a_non_matching_subtree_drops_whole() {
        let (groups, rows) = fixture();
        let nodes = build(&groups, &rows);

        let kept = surviving(&nodes, "kick");
        assert_eq!(names(&nodes, &kept), ["assets", "kick.wav"]);
    }

    #[test]
    fn the_match_is_a_case_insensitive_substring() {
        let (groups, rows) = fixture();
        let nodes = build(&groups, &rows);

        assert_eq!(
            names(&nodes, &surviving(&nodes, "MAIN")),
            ["main.rs", "src"]
        );
        assert_eq!(
            names(&nodes, &surviving(&nodes, "ain.r")),
            ["main.rs", "src"]
        );
    }

    #[test]
    fn an_empty_query_keeps_everything() {
        let (groups, rows) = fixture();
        let nodes = build(&groups, &rows);

        assert_eq!(surviving(&nodes, "").len(), nodes.len());
        assert_eq!(
            surviving(&nodes, "   ").len(),
            nodes.len(),
            "blank is empty"
        );
    }

    #[test]
    fn nothing_matching_keeps_nothing() {
        let (groups, rows) = fixture();
        let nodes = build(&groups, &rows);

        assert!(surviving(&nodes, "zzzz").is_empty());
    }

    #[test]
    fn a_parent_cycle_terminates() {
        // A scanner can emit one; the filter must not hang on it.
        let groups = [gid("a"), gid("b")];
        let nodes = vec![
            FilterNode {
                name: "a",
                group: Some(&groups[0]),
                parent: Some(&groups[1]),
            },
            FilterNode {
                name: "hit",
                group: Some(&groups[1]),
                parent: Some(&groups[0]),
            },
        ];

        let kept = surviving(&nodes, "hit");
        assert_eq!(names(&nodes, &kept), ["a", "hit"]);
    }
}
