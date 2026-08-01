//! The crate against a real world: declaration components in, rows out.
//!
//! Everything else in this crate is a pure function tested without an `App`.
//! What is left to check here is the seam — that [`TreeQuery`] reads the
//! components a host's scanner writes, and keeps two trees apart.

use bevy::prelude::*;
use shellie_tree::{
    DefaultOpen, GroupId, ParentGroup, TreeId, TreeItem, TreeName, TreeQuery, TreeState,
};

/// Spawn an item into the tree `tree`.
fn item(world: &mut World, tree: &str, name: &str) -> Entity {
    world
        .spawn((
            TreeItem,
            TreeId(tree.to_string()),
            TreeName(name.to_string()),
        ))
        .id()
}

fn in_group(world: &mut World, entity: Entity, parent: &str) {
    world.entity_mut(entity).insert(ParentGroup(parent.into()));
}

fn as_group(world: &mut World, entity: Entity, id: &str, default_open: bool) {
    world.entity_mut(entity).insert(GroupId(id.to_string()));
    if default_open {
        world.entity_mut(entity).insert(DefaultOpen);
    }
}

/// Run `TreeQuery::rows` and map back to names, prefixed by depth.
fn rows_of(app: &mut App, tree: &str, query: &str) -> Vec<String> {
    let id = TreeId(tree.to_string());
    let state = app
        .world_mut()
        .query::<(&TreeId, &TreeState)>()
        .iter(app.world())
        .find(|(view, _)| **view == id)
        .map(|(_, s)| s.clone())
        .unwrap_or_default();

    let mut system_state: bevy::ecs::system::SystemState<TreeQuery> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let tree_query = system_state.get(app.world()).expect("query is valid");
    let rows = tree_query.rows(&id, &state, query);

    rows.into_iter()
        .map(|row| {
            let name = app.world().get::<TreeName>(row.item).expect("item alive");
            format!("{}{}", "  ".repeat(row.depth as usize), name.0)
        })
        .collect()
}

/// `src/ { widgets/ { button.rs }, main.rs }` plus a root leaf.
fn app_with_tree() -> App {
    let mut app = App::new();
    let world = app.world_mut();

    world.spawn((TreeId("browser".to_string()), TreeState::new()));

    let src = item(world, "browser", "src");
    as_group(world, src, "src", true);

    let widgets = item(world, "browser", "widgets");
    as_group(world, widgets, "widgets", true);
    in_group(world, widgets, "src");

    let button = item(world, "browser", "button.rs");
    in_group(world, button, "widgets");

    let main = item(world, "browser", "main.rs");
    in_group(world, main, "src");

    item(world, "browser", "Cargo.toml");

    app
}

#[test]
fn declaration_components_become_rows() {
    let mut app = app_with_tree();

    assert_eq!(
        rows_of(&mut app, "browser", ""),
        [
            "src",
            "  widgets",
            "    button.rs",
            "  main.rs",
            "Cargo.toml"
        ]
    );
}

#[test]
fn an_entity_without_the_marker_is_invisible() {
    // A host can park a half-built item in the world without it flickering
    // into the list.
    let mut app = app_with_tree();
    app.world_mut().spawn((
        TreeId("browser".to_string()),
        TreeName("half-built".to_string()),
    ));

    assert!(
        !rows_of(&mut app, "browser", "")
            .iter()
            .any(|r| r.contains("half-built"))
    );
}

#[test]
fn two_trees_in_one_world_stay_apart() {
    let mut app = app_with_tree();
    {
        let world = app.world_mut();
        world.spawn((TreeId("outline".to_string()), TreeState::new()));
        item(world, "outline", "Chapter One");
    }

    assert_eq!(rows_of(&mut app, "outline", ""), ["Chapter One"]);
    assert!(
        !rows_of(&mut app, "browser", "")
            .iter()
            .any(|r| r.contains("Chapter"))
    );
}

#[test]
fn two_trees_have_independent_expansion() {
    // The bug a single global state resource makes unrepresentable: closing a
    // group in one tree must not close the same-named group in another.
    let mut app = App::new();
    {
        let world = app.world_mut();
        for tree in ["left", "right"] {
            world.spawn((TreeId(tree.to_string()), TreeState::new()));
            let g = item(world, tree, "shared-name");
            as_group(world, g, "shared", true);
            let child = item(world, tree, "child");
            in_group(world, child, "shared");
        }

        // Close it in `left` only.
        let mut views = world.query::<(&TreeId, &mut TreeState)>();
        for (id, mut state) in views.iter_mut(world) {
            if id.as_str() == "left" {
                state.toggle(&GroupId("shared".into()), true);
            }
        }
    }

    assert_eq!(rows_of(&mut app, "left", ""), ["shared-name"]);
    assert_eq!(rows_of(&mut app, "right", ""), ["shared-name", "  child"]);
}

#[test]
fn a_section_is_just_a_root_group() {
    // No separate section type, no second open/closed mechanism: a top-level
    // collapsible band is a group with no parent.
    let mut app = App::new();
    {
        let world = app.world_mut();
        world.spawn((TreeId("browser".to_string()), TreeState::new()));

        let devices = item(world, "browser", "DEVICES");
        as_group(world, devices, "devices", true);
        let midi = item(world, "browser", "MIDI Keyboard");
        in_group(world, midi, "devices");

        let effects = item(world, "browser", "EFFECTS");
        as_group(world, effects, "effects", false);
        let reverb = item(world, "browser", "Reverb");
        in_group(world, reverb, "effects");
    }

    // `devices` declared open, `effects` declared closed — one mechanism.
    assert_eq!(
        rows_of(&mut app, "browser", ""),
        ["DEVICES", "  MIDI Keyboard", "EFFECTS"]
    );
}

#[test]
fn a_search_finds_a_leaf_under_a_non_matching_group() {
    // The defect this crate replaces: filtering item-by-item drops `src`,
    // because its name is not "button", and the match goes down with it.
    let mut app = app_with_tree();

    assert_eq!(
        rows_of(&mut app, "browser", "button"),
        ["src", "  widgets", "    button.rs"]
    );
}

#[test]
fn a_despawned_item_leaves_a_stale_entity_in_the_row() {
    // `VisibleRow::item` is a hint. Extension catalogs replace their items
    // wholesale, so a host must tolerate a row pointing at nothing rather
    // than index it blind.
    let mut app = app_with_tree();
    let id = TreeId("browser".to_string());
    let state = TreeState::new();

    let rows = {
        let mut system_state: bevy::ecs::system::SystemState<TreeQuery> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let tree_query = system_state.get(app.world()).expect("query is valid");
        tree_query.rows(&id, &state, "")
    };

    let victim = rows[0].item;
    app.world_mut().entity_mut(victim).despawn();

    assert!(app.world().get_entity(victim).is_err());
    assert!(
        rows.iter().any(|r| r.item == victim),
        "the computed list still names it — hosts must check before dereferencing"
    );
}

#[test]
fn is_empty_reports_whether_a_scan_has_produced_anything() {
    let mut app = app_with_tree();
    let mut system_state: bevy::ecs::system::SystemState<TreeQuery> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let tree_query = system_state.get(app.world()).expect("query is valid");

    assert!(!tree_query.is_empty(&TreeId("browser".into())));
    assert!(tree_query.is_empty(&TreeId("never-scanned".into())));
}
