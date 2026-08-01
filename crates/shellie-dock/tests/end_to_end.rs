//! End-to-end checks through a real `App`.
//!
//! The unit tests cover each stage alone; these exist for the seams between
//! them. A layout that reconciles correctly but is applied at the wrong point
//! in the frame, or a mutation that edits the tree without asking for a
//! rebuild, passes every unit test and still leaves the user staring at a
//! stale dock.

use bevy::prelude::*;
use shellie_dock::registry::Panes;
use shellie_dock::{
    ActivePage, Axis, DockCommands, DockLayout, DockTree, Page, PageId, PaneRegistry,
    PaneVisibility, ShellieDockPlugin, Side, page_is_active, pane_exists,
};

#[derive(Component)]
struct MyPanel;

fn test_app(layout: DockLayout) -> App {
    let mut app = App::new();
    app.add_plugins(ShellieDockPlugin).insert_resource(layout);
    app
}

fn ide_shaped_tree() -> DockLayout {
    DockLayout::new(DockTree::split(
        Axis::Column,
        [
            DockTree::pane("title_bar").fixed(40.0),
            DockTree::split(
                Axis::Row,
                [
                    DockTree::pane("browser").flex(1.0),
                    DockTree::pane("center").flex(6.0),
                    DockTree::pane("inspector").flex(1.0).min_size(120.0),
                ],
            )
            .flex(1.0),
        ],
    ))
}

fn pane_ids(layout: &DockLayout) -> Vec<String> {
    layout.pane_ids().map(|p| p.0.clone()).collect()
}

#[test]
fn a_declared_tree_is_queryable_after_one_frame() {
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let registry = app.world().resource::<PaneRegistry>();
    assert_eq!(registry.len(), 4);
    for id in ["title_bar", "browser", "center", "inspector"] {
        assert!(registry.get(id).is_some(), "{id} should be findable");
    }
}

#[test]
fn content_finds_its_pane_by_id() {
    // The poll-until-it-exists pattern a real host uses: poll until the pane exists, then
    // parent in. The dock never pushes content, so this is the contract every
    // panel in every consuming crate depends on.
    fn spawn_panel_once(mut commands: Commands, existing: Query<(), With<MyPanel>>, panes: Panes) {
        if !existing.is_empty() {
            return;
        }
        let Some(pane) = panes.get("browser") else {
            return;
        };
        commands.spawn((MyPanel, ChildOf(pane)));
    }

    let mut app = test_app(ide_shaped_tree());
    app.add_systems(Update, spawn_panel_once);
    // Two frames, deliberately. The panel is unordered against the build, so on
    // the first frame the pane may not exist yet — which is precisely why the
    // contract is "poll until it appears" rather than "spawn once at startup".
    app.update();
    app.update();

    let pane = app
        .world()
        .resource::<PaneRegistry>()
        .get("browser")
        .unwrap();
    let mut panels = app.world_mut().query_filtered::<&ChildOf, With<MyPanel>>();
    let parents: Vec<Entity> = panels.iter(app.world()).map(ChildOf::parent).collect();
    assert_eq!(parents, [pane]);
}

#[test]
fn a_panel_gated_on_pane_exists_does_not_run_early() {
    #[derive(Resource, Default)]
    struct Ran(u32);

    fn count(mut ran: ResMut<Ran>) {
        ran.0 += 1;
    }

    let mut app = App::new();
    app.add_plugins(ShellieDockPlugin)
        .insert_resource(DockLayout::new(DockTree::pane("late")))
        .init_resource::<Ran>()
        .add_systems(Update, count.run_if(pane_exists("nonexistent")));
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<Ran>().0,
        0,
        "a condition on a pane that never appears must never open"
    );
}

#[test]
fn capture_then_apply_round_trips_through_a_world() {
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let captured = DockLayout::capture(app.world()).expect("a built layout");

    // A fresh app declaring the same tree adopts the saved one.
    let mut restored = test_app(ide_shaped_tree());
    restored.update();
    captured.apply(restored.world_mut());
    restored.update();

    assert_eq!(
        pane_ids(restored.world().resource::<DockLayout>()),
        pane_ids(&captured)
    );
    assert_eq!(restored.world().resource::<PaneRegistry>().len(), 4);
}

#[test]
fn a_saved_layout_survives_a_serde_round_trip() {
    // The host writes RON; this crate picks no format, so the test proves the
    // shape is serializable rather than that any particular encoder works.
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let captured = DockLayout::capture(app.world()).expect("a built layout");
    let encoded = ron::ser::to_string(&captured).expect("serializes");
    let decoded: DockLayout = ron::from_str(&encoded).expect("round-trips");

    assert_eq!(decoded, captured);
}

#[test]
fn a_layout_saved_without_a_version_still_loads() {
    // The field defaults rather than failing: per-machine layout state must
    // never be the reason a project refuses to open.
    let ron = r#"(root: Pane(id: ("only"), size: Flex(1.0), min_size: None, visibility: Shown))"#;
    let decoded: DockLayout = ron::from_str(ron).expect("a versionless file still parses");
    assert_eq!(pane_ids(&decoded), ["only"]);
}

#[test]
fn a_saved_layout_missing_a_declared_pane_gains_it_back() {
    // The app grew a panel since the file was written.
    let saved = DockLayout::new(DockTree::split(
        Axis::Row,
        [DockTree::pane("browser"), DockTree::pane("center")],
    ));

    let mut app = test_app(ide_shaped_tree());
    app.update();
    saved.apply(app.world_mut());
    app.update();

    let registry = app.world().resource::<PaneRegistry>();
    assert!(registry.get("inspector").is_some(), "the new pane appears");
    assert!(registry.get("title_bar").is_some());
    assert!(registry.get("browser").is_some());
}

#[test]
fn a_host_can_prune_a_retired_pane_before_applying() {
    // The dock keeps saved panes it does not recognise, because it cannot tell
    // a retired panel from one the user created. A host that *does* know says
    // so explicitly.
    let mut saved = DockLayout::new(DockTree::split(
        Axis::Row,
        [
            DockTree::pane("browser"),
            DockTree::pane("center"),
            DockTree::pane("retired_panel"),
        ],
    ));

    let mut app = test_app(ide_shaped_tree());
    app.update();

    assert_eq!(
        saved.retain_panes(|id| id != "retired_panel"),
        ["retired_panel"]
    );
    saved.apply(app.world_mut());
    app.update();

    let registry = app.world().resource::<PaneRegistry>();
    assert!(registry.get("retired_panel").is_none());
    assert!(registry.get("browser").is_some(), "the rest survives");
}

#[test]
fn a_runtime_split_survives_capture_and_apply() {
    // Proves mutation and persistence agree: a pane the user created at
    // runtime has to come back after a save/load like any declared one.
    fn split_the_center(mut dock: DockCommands) {
        dock.split_pane("center", Axis::Column, "console", Side::After);
    }

    let mut app = test_app(ide_shaped_tree());
    app.update();
    app.world_mut().run_system_once(split_the_center).unwrap();
    app.update();

    assert!(
        app.world()
            .resource::<PaneRegistry>()
            .get("console")
            .is_some()
    );

    let captured = DockLayout::capture(app.world()).expect("a built layout");
    let encoded = ron::ser::to_string(&captured).unwrap();
    let decoded: DockLayout = ron::from_str(&encoded).unwrap();

    // The reloading app declares the *original* tree — the console pane exists
    // only in the save, which is exactly the situation after a restart.
    let mut restored = test_app(ide_shaped_tree());
    restored.update();
    decoded.apply(restored.world_mut());
    restored.update();

    assert!(
        restored
            .world()
            .resource::<PaneRegistry>()
            .get("console")
            .is_some(),
        "a runtime-created pane must survive a save/load"
    );
}

use bevy::ecs::system::RunSystemOnce;

#[test]
fn a_runtime_mutation_rebuilds_without_a_layout_swap() {
    // `DockCommands` edits the layout in place. If the rebuild were driven by
    // resource change detection alone this would still fire, but the mutation
    // must not depend on that -- hence the explicit request.
    fn remove_the_browser(mut dock: DockCommands) {
        dock.remove_pane("browser");
    }

    let mut app = test_app(ide_shaped_tree());
    app.update();
    assert!(
        app.world()
            .resource::<PaneRegistry>()
            .get("browser")
            .is_some()
    );

    app.world_mut().run_system_once(remove_the_browser).unwrap();
    app.update();

    let registry = app.world().resource::<PaneRegistry>();
    assert!(registry.get("browser").is_none());
    assert_eq!(registry.len(), 3);
    assert_eq!(
        app.world().resource::<DockLayout>().validate(),
        Ok(()),
        "a mutation must leave a tree the crate would accept"
    );
}

#[test]
fn hiding_a_pane_leaves_it_in_the_tree() {
    // Hidden is not removed: the pane keeps its entity, its content and its
    // size, so unhiding restores what the user had rather than a default.
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let browser = app
        .world()
        .resource::<PaneRegistry>()
        .get("browser")
        .unwrap();
    let content = app.world_mut().spawn((MyPanel, ChildOf(browser))).id();

    app.world_mut()
        .entity_mut(browser)
        .insert(PaneVisibility::Hidden);
    app.update();

    assert_eq!(
        app.world().get::<Node>(browser).unwrap().display,
        Display::None
    );
    assert!(app.world().get_entity(content).is_ok());
    assert!(
        app.world()
            .resource::<PaneRegistry>()
            .get("browser")
            .is_some()
    );

    app.world_mut()
        .entity_mut(browser)
        .insert(PaneVisibility::Shown);
    app.update();

    assert_eq!(
        app.world().get::<Node>(browser).unwrap().display,
        Display::Flex
    );
}

/// Spawn `ids` as pages of `pane`, with the first active.
fn add_pages(app: &mut App, pane: Entity, ids: &[&str]) -> Vec<Entity> {
    let pages = ids
        .iter()
        .map(|id| {
            app.world_mut()
                .spawn((Page, PageId::from(*id), Node::default(), ChildOf(pane)))
                .id()
        })
        .collect();
    if let Some(first) = ids.first() {
        app.world_mut()
            .entity_mut(pane)
            .insert(ActivePage::at(*first));
    }
    pages
}

fn display_of(app: &App, entity: Entity) -> Display {
    app.world().get::<Node>(entity).unwrap().display
}

#[test]
fn only_the_active_page_takes_part_in_layout() {
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let center = app
        .world()
        .resource::<PaneRegistry>()
        .get("center")
        .unwrap();
    let pages = add_pages(&mut app, center, &["timeline", "spectral"]);
    app.update();

    assert_eq!(display_of(&app, pages[0]), Display::Flex);
    assert_eq!(
        display_of(&app, pages[1]),
        Display::None,
        "an inactive page must measure zero, not merely be invisible"
    );
}

#[test]
fn switching_pages_swaps_which_one_shows() {
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let center = app
        .world()
        .resource::<PaneRegistry>()
        .get("center")
        .unwrap();
    let pages = add_pages(&mut app, center, &["timeline", "spectral"]);
    app.update();

    app.world_mut()
        .get_mut::<ActivePage>(center)
        .unwrap()
        .set("spectral");
    app.update();

    assert_eq!(display_of(&app, pages[0]), Display::None);
    assert_eq!(display_of(&app, pages[1]), Display::Flex);
}

#[test]
fn two_panes_hold_pages_independently() {
    // The property a single global "active mode" resource cannot express, and
    // the reason `ActivePage` lives on the pane.
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let (center, browser) = {
        let registry = app.world().resource::<PaneRegistry>();
        (
            registry.get("center").unwrap(),
            registry.get("browser").unwrap(),
        )
    };
    let center_pages = add_pages(&mut app, center, &["timeline", "spectral"]);
    let browser_pages = add_pages(&mut app, browser, &["files", "presets"]);
    app.update();

    app.world_mut()
        .get_mut::<ActivePage>(center)
        .unwrap()
        .set("spectral");
    app.update();

    assert_eq!(display_of(&app, center_pages[1]), Display::Flex);
    assert_eq!(
        display_of(&app, browser_pages[0]),
        Display::Flex,
        "the other pane's active page is untouched"
    );
    assert_eq!(display_of(&app, browser_pages[1]), Display::None);
}

#[test]
fn a_page_added_later_is_hidden_without_a_switch() {
    // A page spawning into an already-settled pane must not appear on top of
    // the active one just because `ActivePage` did not change.
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let center = app
        .world()
        .resource::<PaneRegistry>()
        .get("center")
        .unwrap();
    add_pages(&mut app, center, &["timeline"]);
    app.update();

    let late = app
        .world_mut()
        .spawn((
            Page,
            PageId::from("spectral"),
            Node::default(),
            ChildOf(center),
        ))
        .id();
    app.update();

    assert_eq!(display_of(&app, late), Display::None);
}

#[test]
fn a_page_is_inert_while_another_is_active() {
    #[derive(Resource, Default)]
    struct Ran(u32);

    fn page_system(mut ran: ResMut<Ran>) {
        ran.0 += 1;
    }

    let mut app = test_app(ide_shaped_tree());
    app.init_resource::<Ran>()
        .add_systems(Update, page_system.run_if(page_is_active("timeline")));
    app.update();

    let center = app
        .world()
        .resource::<PaneRegistry>()
        .get("center")
        .unwrap();
    add_pages(&mut app, center, &["spectral"]);
    app.update();
    assert_eq!(app.world().resource::<Ran>().0, 0);

    app.world_mut()
        .get_mut::<ActivePage>(center)
        .unwrap()
        .set("timeline");
    app.update();
    assert_eq!(app.world().resource::<Ran>().0, 1);
}

#[test]
fn pages_survive_a_layout_change_with_their_pane() {
    // Pages are children of the pane frame, so the reconcile that preserves a
    // pane preserves its pages — no re-registration on every resize.
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let center = app
        .world()
        .resource::<PaneRegistry>()
        .get("center")
        .unwrap();
    let pages = add_pages(&mut app, center, &["timeline", "spectral"]);

    app.world_mut()
        .insert_resource(DockLayout::new(DockTree::split(
            Axis::Row,
            [
                DockTree::pane("browser").flex(3.0),
                DockTree::pane("center").flex(5.0),
            ],
        )));
    app.update();

    assert!(app.world().get_entity(pages[0]).is_ok());
    assert_eq!(
        app.world().get::<ChildOf>(pages[0]).map(ChildOf::parent),
        Some(center),
    );
    assert!(
        app.world()
            .get::<ActivePage>(center)
            .unwrap()
            .is("timeline")
    );
}

#[test]
fn pages_work_with_no_dock_present() {
    // Demo binaries and test harnesses hang pages off one full-window node and
    // switch between them. Nothing in the page vocabulary may reach for a pane,
    // or a page's own crate cannot be tested without standing up a layout.
    // No `ShellieDockPlugin` at all — just the one system, which is the point:
    // a page's crate must be testable without standing up a layout.
    let mut app = App::new();
    app.add_systems(Update, shellie_dock::page::apply_active_page);

    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ActivePage::at("demo"),
        ))
        .id();
    let pages: Vec<Entity> = ["demo", "other"]
        .iter()
        .map(|id| {
            app.world_mut()
                .spawn((Page, PageId::from(*id), Node::default(), ChildOf(container)))
                .id()
        })
        .collect();
    app.update();

    assert_eq!(display_of(&app, pages[0]), Display::Flex);
    assert_eq!(display_of(&app, pages[1]), Display::None);
    assert!(
        app.world().get::<shellie_dock::Pane>(container).is_none(),
        "no pane was involved, and none was needed"
    );
}

#[test]
fn a_dock_with_no_pages_carries_no_page_state() {
    // Pages are opt-in: an app that never uses them pays nothing, which is why
    // the plugin initializes no page resource.
    let mut app = test_app(ide_shaped_tree());
    app.update();

    let panes_with_pages = app
        .world_mut()
        .query::<&ActivePage>()
        .iter(app.world())
        .count();
    assert_eq!(panes_with_pages, 0);
}
