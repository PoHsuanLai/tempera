//! What [`visible_rows`] emits, and in what order.
//!
//! These need no `App`: the computation is a pure function over a slice, which
//! is most of the argument for the flat-list model.

use bevy::prelude::Entity;
use tempera_tree::{
    GroupId, TreeNode, TreeState, VisibleRow, groups_first_then_name, visible_rows, visible_rows_by,
};

/// A fixture row before its borrows are taken.
struct Row {
    name: &'static str,
    group: Option<GroupId>,
    parent: Option<GroupId>,
    default_open: bool,
}

fn leaf(name: &'static str, parent: Option<&str>) -> Row {
    Row {
        name,
        group: None,
        parent: parent.map(|p| GroupId(p.to_string())),
        default_open: false,
    }
}

fn group(name: &'static str, id: &str, parent: Option<&str>, default_open: bool) -> Row {
    Row {
        name,
        group: Some(GroupId(id.to_string())),
        parent: parent.map(|p| GroupId(p.to_string())),
        default_open,
    }
}

fn nodes(rows: &[Row]) -> Vec<TreeNode<'_>> {
    rows.iter()
        .enumerate()
        .map(|(i, r)| TreeNode {
            entity: Entity::from_raw_u32(i as u32).expect("valid entity index"),
            name: r.name,
            group: r.group.as_ref(),
            parent: r.parent.as_ref(),
            default_open: r.default_open,
        })
        .collect()
}

/// Row names in emitted order, each prefixed by its depth.
fn shape(rows: &[Row], out: &[VisibleRow]) -> Vec<String> {
    out.iter()
        .map(|r| {
            let name = rows
                .iter()
                .enumerate()
                .find(|(i, _)| {
                    Entity::from_raw_u32(*i as u32).expect("valid entity index") == r.item
                })
                .map(|(_, row)| row.name)
                .expect("row belongs to the fixture");
            format!("{}{}", "  ".repeat(r.depth as usize), name)
        })
        .collect()
}

/// `src/ { main.rs, widgets/ { button.rs } }`, `assets/ { kick.wav }`,
/// `Cargo.toml`. Every group defaults open.
fn tree() -> Vec<Row> {
    vec![
        group("src", "src", None, true),
        leaf("main.rs", Some("src")),
        group("widgets", "widgets", Some("src"), true),
        leaf("button.rs", Some("widgets")),
        group("assets", "assets", None, true),
        leaf("kick.wav", Some("assets")),
        leaf("Cargo.toml", None),
    ]
}

#[test]
fn depth_increments_once_per_level() {
    let rows = tree();
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    // Groups before leaves applies at every level, so under `src` the
    // `widgets` group precedes the `main.rs` leaf.
    assert_eq!(
        shape(&rows, &out),
        [
            "assets",
            "  kick.wav",
            "src",
            "  widgets",
            "    button.rs",
            "  main.rs",
            "Cargo.toml",
        ]
    );
}

#[test]
fn groups_sort_before_leaves_then_alphabetically() {
    let rows = vec![
        leaf("zebra", None),
        leaf("apple", None),
        group("Widgets", "w", None, false),
        group("assets", "a", None, false),
    ];
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    // Case-insensitive: "assets" before "Widgets", both before the leaves.
    assert_eq!(shape(&rows, &out), ["assets", "Widgets", "apple", "zebra"]);
}

#[test]
fn a_collapsed_group_omits_its_descendants_entirely() {
    // Not "renders them hidden" — absent from the list. That is the whole
    // point of the flat model: a collapsed subtree costs nothing.
    let rows = tree();
    let mut state = TreeState::new();
    state.toggle(&GroupId("src".into()), true);

    let out = visible_rows(&nodes(&rows), &state, "");
    assert_eq!(
        shape(&rows, &out),
        ["assets", "  kick.wav", "src", "Cargo.toml"]
    );
    let src = out.iter().find(|r| r.is_group && !r.expanded);
    assert!(src.is_some(), "the collapsed group itself still renders");
}

#[test]
fn collapsing_a_parent_hides_a_nested_group_and_its_children() {
    let rows = tree();
    let mut state = TreeState::new();
    state.toggle(&GroupId("src".into()), true);

    let out = visible_rows(&nodes(&rows), &state, "");
    let names = shape(&rows, &out);
    assert!(!names.iter().any(|n| n.trim() == "widgets"));
    assert!(!names.iter().any(|n| n.trim() == "button.rs"));
}

#[test]
fn a_default_closed_group_starts_closed() {
    let rows = vec![
        group("target", "target", None, false),
        leaf("debug", Some("target")),
    ];
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    assert_eq!(shape(&rows, &out), ["target"]);
    assert!(!out[0].expanded);
}

#[test]
fn a_query_flattens_without_mutating_state() {
    // Typing must not rewrite the user's expansion as a side effect.
    let rows = tree();
    let mut state = TreeState::new();
    state.toggle(&GroupId("src".into()), true);
    let before = state.clone();

    let searching = visible_rows(&nodes(&rows), &state, "button");
    assert_eq!(
        shape(&rows, &searching),
        ["src", "  widgets", "    button.rs"]
    );

    // Clearing the query restores exactly what the user had.
    let after = visible_rows(&nodes(&rows), &state, "");
    assert_eq!(
        shape(&rows, &after),
        ["assets", "  kick.wav", "src", "Cargo.toml"]
    );
    assert_eq!(before.len(), state.len(), "the query left state alone");
}

#[test]
fn a_search_reaches_into_a_collapsed_group() {
    // The user closed `src`; searching for something inside it still finds it.
    let rows = tree();
    let mut state = TreeState::new();
    state.toggle(&GroupId("src".into()), true);

    let out = visible_rows(&nodes(&rows), &state, "main");
    assert_eq!(shape(&rows, &out), ["src", "  main.rs"]);
}

#[test]
fn an_item_naming_a_nonexistent_parent_is_dropped_without_taking_siblings() {
    let rows = vec![
        group("src", "src", None, true),
        leaf("main.rs", Some("src")),
        leaf("orphan.rs", Some("no-such-group")),
    ];
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    assert_eq!(shape(&rows, &out), ["src", "  main.rs"]);
}

#[test]
fn a_duplicate_group_id_keeps_the_first_claimant() {
    // The id is how a parent link and the saved state both name a group, so
    // two claimants make "is src open" ambiguous. One wins, deterministically.
    let rows = vec![
        group("first", "src", None, true),
        group("second", "src", None, true),
        leaf("main.rs", Some("src")),
    ];
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    assert_eq!(shape(&rows, &out), ["first", "  main.rs"]);
}

#[test]
fn a_parent_cycle_terminates() {
    // A scanner can emit one. The walk must stop rather than recurse forever.
    let rows = vec![
        group("a", "a", Some("b"), true),
        group("b", "b", Some("a"), true),
    ];
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    // Neither is a root, so nothing is reachable — but the call returns.
    assert!(out.is_empty());
}

#[test]
fn a_self_parented_group_terminates() {
    let rows = vec![
        group("root", "root", None, true),
        group("loop", "loop", Some("loop"), true),
        leaf("leaf", Some("root")),
    ];
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    assert_eq!(shape(&rows, &out), ["root", "  leaf"]);
}

#[test]
fn an_empty_tree_is_an_empty_list() {
    assert!(visible_rows(&[], &TreeState::new(), "").is_empty());
    assert!(visible_rows(&[], &TreeState::new(), "query").is_empty());
}

#[test]
fn a_query_matching_nothing_emits_nothing() {
    let rows = tree();
    assert!(visible_rows(&nodes(&rows), &TreeState::new(), "zzzz").is_empty());
}

#[test]
fn expanded_is_reported_per_group() {
    let rows = tree();
    let mut state = TreeState::new();
    state.toggle(&GroupId("assets".into()), true);

    let out = visible_rows(&nodes(&rows), &state, "");
    let by_name: Vec<(String, bool, bool)> = out
        .iter()
        .zip(shape(&rows, &out))
        .map(|(r, n)| (n.trim().to_string(), r.is_group, r.expanded))
        .collect();

    assert!(by_name.contains(&("assets".into(), true, false)));
    assert!(by_name.contains(&("src".into(), true, true)));
    assert!(by_name.contains(&("main.rs".into(), false, false)));
}

#[test]
fn siblings_at_depth_three_keep_their_own_depth() {
    // Guards the walk's depth bookkeeping: a deep branch must not leak its
    // depth onto whatever is emitted after it.
    let rows = vec![
        group("a", "a", None, true),
        group("b", "b", Some("a"), true),
        group("c", "c", Some("b"), true),
        leaf("deep", Some("c")),
        leaf("shallow", Some("a")),
        leaf("root-leaf", None),
    ];
    let out = visible_rows(&nodes(&rows), &TreeState::new(), "");

    assert_eq!(
        shape(&rows, &out),
        ["a", "  b", "    c", "      deep", "  shallow", "root-leaf"]
    );
}

// ---------------------------------------------------------------------------
// Sibling order is the caller's
// ---------------------------------------------------------------------------

/// Reverse-alphabetical, ignoring the group/leaf split entirely — deliberately
/// nothing like the default, so a comparator that is quietly ignored cannot
/// coincide with the right answer.
fn by_name_descending(a: &TreeNode<'_>, b: &TreeNode<'_>) -> std::cmp::Ordering {
    b.name.to_lowercase().cmp(&a.name.to_lowercase())
}

#[test]
fn a_caller_can_choose_the_sibling_order() {
    // The property `visible_rows_by` exists for. Before it, a host wanting a
    // different order had to prefix its names — visible to the user *and*
    // matched by the search query.
    let rows = [leaf("alpha", None), leaf("beta", None), leaf("gamma", None)];
    let out = visible_rows_by(
        &nodes(&rows),
        &TreeState::default(),
        "",
        &by_name_descending,
    );
    assert_eq!(shape(&rows, &out), ["gamma", "beta", "alpha"]);
}

#[test]
fn the_order_applies_inside_a_group_too() {
    // Not just at the roots. A comparator that only reached the top level would
    // pass the test above and still leave every group's contents in the default
    // order, which is the half nobody would notice until a deep tree.
    let rows = [
        group("SAMPLES", "samples", None, true),
        leaf("alpha", Some("samples")),
        leaf("beta", Some("samples")),
    ];
    let out = visible_rows_by(
        &nodes(&rows),
        &TreeState::default(),
        "",
        &by_name_descending,
    );
    assert_eq!(shape(&rows, &out), ["SAMPLES", "  beta", "  alpha"]);
}

#[test]
fn a_comparator_cannot_move_a_child_out_of_its_group() {
    // The bound on what a caller can do: order is applied *within* one parent,
    // so the hierarchy is not negotiable. A comparator that sorted a leaf above
    // its own group would otherwise produce a row whose depth contradicts its
    // position.
    let rows = [group("ZZZ", "zzz", None, true), leaf("aaa", Some("zzz"))];
    let out = visible_rows_by(
        &nodes(&rows),
        &TreeState::default(),
        "",
        &by_name_descending,
    );
    assert_eq!(
        shape(&rows, &out),
        ["ZZZ", "  aaa"],
        "a child must stay under its parent whatever the comparator says"
    );
}

#[test]
fn the_default_is_still_groups_first_then_name() {
    // `visible_rows` delegates now, so this pins that the delegation kept the
    // old policy rather than silently adopting a new one. Every existing call
    // site depends on it.
    let rows = [leaf("aaa", None), group("zzz", "zzz", None, false)];
    let by_default = visible_rows(&nodes(&rows), &TreeState::default(), "");
    let explicit = visible_rows_by(
        &nodes(&rows),
        &TreeState::default(),
        "",
        &groups_first_then_name,
    );
    assert_eq!(shape(&rows, &by_default), ["zzz", "aaa"]);
    assert_eq!(shape(&rows, &by_default), shape(&rows, &explicit));
}
