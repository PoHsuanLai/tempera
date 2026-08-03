//! The crate through a real `App`: declared tabs in, a working dialog out.
//!
//! The unit tests cover the value types alone; these exist for the seams —
//! a tab declared after the dialog was built, a body that must hide itself
//! without clobbering its own layout, two dialogs that must not share a
//! selection.

use bevy::prelude::*;
use tempera_settings::{
    ActiveTab, SettingsBody, SettingsDialog, SettingsOpen, SettingsTab, SidebarEntry, TabBodies,
    TabBody, TabId, TabLabel, TabOrder, TemperaSettingsPlugin, tab_exists,
};

/// An app with the theme resources tempera needs and a loaded font, so the
/// build does not park waiting for one.
fn test_app() -> App {
    let mut app = App::new();
    // `ThemePlugin` rather than three hand-rolled `init_resource` calls: the
    // token set grows (it gained `Tokens` and `ThemeConfig` when the theme
    // became a function), and a test that lists the resources by hand goes
    // from green to a `SystemParam` panic the moment a widget reads a new
    // one. The plugin is idempotent, so this composes with whatever the
    // settings plugin already installed.
    app.add_plugins((TemperaSettingsPlugin, tempera::ThemePlugin))
        .init_resource::<Assets<Font>>()
        // Registered here rather than by the plugin: input is the host's,
        // and the crate treats the wheel as optional precisely so an app
        // without `InputPlugin` still runs.
        .add_message::<bevy::input::mouse::MouseWheel>();

    let handle = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .reserve_handle();
    app.insert_resource(tempera::FontHandle {
        regular: Some(handle),
        ..default()
    });
    app
}

fn declare(app: &mut App, id: &str, label: &str, order: i32) -> Entity {
    app.world_mut()
        .spawn((
            SettingsTab,
            TabId::from(id),
            TabLabel::from(label),
            TabOrder(order),
        ))
        .id()
}

fn body_of(app: &mut App, id: &str) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &TabId, &TabBody)>();
    q.iter(app.world())
        .find(|(_, tab_id, _)| tab_id.as_str() == id)
        .map(|(e, _, _)| e)
}

fn sidebar_ids(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&SidebarEntry>();
    q.iter(app.world()).map(|e| e.0.0.clone()).collect()
}

fn display_of(app: &App, entity: Entity) -> Display {
    app.world().get::<Node>(entity).unwrap().display
}

fn dialog(app: &mut App) -> Entity {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<SettingsDialog>>();
    q.iter(app.world()).next().expect("a dialog was built")
}

/// One wheel line downward.
fn scroll_down(app: &mut App) {
    app.world_mut()
        .write_message(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
}

#[test]
fn declared_tabs_become_a_sidebar_and_bodies() {
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    declare(&mut app, "audio", "Audio", 20);
    app.update();
    app.update();

    assert_eq!(sidebar_ids(&mut app), ["general", "audio"]);
    assert!(body_of(&mut app, "general").is_some());
    assert!(body_of(&mut app, "audio").is_some());
}

#[test]
fn tabs_sort_by_order_not_declaration() {
    // Tabs are declared by whichever crates are present; system order is
    // not a contract a host should have to reason about.
    let mut app = test_app();
    declare(&mut app, "zulu", "Zulu", 10);
    declare(&mut app, "alpha", "Alpha", 20);
    app.update();
    app.update();

    assert_eq!(sidebar_ids(&mut app), ["zulu", "alpha"]);
}

#[test]
fn a_tab_declared_after_the_build_still_appears() {
    // The case an extension hits: it registers on frame 900, long after the
    // dialog settled. A spawn-once build would never see it.
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    app.update();
    app.update();
    assert_eq!(sidebar_ids(&mut app).len(), 1);

    declare(&mut app, "late", "Late", 20);
    app.update();

    assert_eq!(sidebar_ids(&mut app), ["general", "late"]);
    assert!(body_of(&mut app, "late").is_some());
}

#[test]
fn a_retired_tab_takes_its_body_and_entry_with_it() {
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    let doomed = declare(&mut app, "doomed", "Doomed", 20);
    app.update();
    app.update();

    let body = body_of(&mut app, "doomed").expect("body exists");
    let content = app.world_mut().spawn(ChildOf(body)).id();

    app.world_mut().entity_mut(doomed).despawn();
    app.update();

    assert_eq!(sidebar_ids(&mut app), ["general"]);
    assert!(body_of(&mut app, "doomed").is_none());
    assert!(
        app.world().get_entity(content).is_err(),
        "a dropped tab takes the host's content with it"
    );
}

#[test]
fn only_the_active_tabs_body_takes_part_in_layout() {
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    declare(&mut app, "audio", "Audio", 20);
    app.update();
    app.update();

    let root = dialog(&mut app);
    app.world_mut()
        .get_mut::<ActiveTab>(root)
        .unwrap()
        .set("audio");
    app.update();

    let general = body_of(&mut app, "general").unwrap();
    let audio = body_of(&mut app, "audio").unwrap();
    assert_eq!(display_of(&app, audio), Display::Flex);
    assert_eq!(
        display_of(&app, general),
        Display::None,
        "an inactive body must measure zero, not merely be invisible"
    );
}

#[test]
fn switching_tabs_preserves_a_bodys_own_layout() {
    // The bug this replaces: the old implementation inserted a whole new
    // `Node` per tab, silently clobbering any layout a body set for itself.
    // One tab's `Overflow::scroll_y()` was dead after the first switch.
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    declare(&mut app, "audio", "Audio", 20);
    app.update();
    app.update();

    let audio = body_of(&mut app, "audio").unwrap();
    app.world_mut().get_mut::<Node>(audio).unwrap().overflow = Overflow::scroll_y();

    let root = dialog(&mut app);
    app.world_mut()
        .get_mut::<ActiveTab>(root)
        .unwrap()
        .set("audio");
    app.update();
    app.world_mut()
        .get_mut::<ActiveTab>(root)
        .unwrap()
        .set("general");
    app.update();

    assert_eq!(
        app.world().get::<Node>(audio).unwrap().overflow,
        Overflow::scroll_y(),
        "hiding a tab must not overwrite the layout it set for itself"
    );
}

#[test]
fn content_finds_its_body_by_id() {
    // The inversion the whole crate rests on: a panel in a crate this one
    // has never heard of polls for its body and parents itself in.
    #[derive(Component)]
    struct MyPanel;

    fn spawn_panel(mut commands: Commands, bodies: TabBodies, mine: Query<(), With<MyPanel>>) {
        if !mine.is_empty() {
            return;
        }
        let Some(body) = bodies.get("audio") else {
            return;
        };
        commands.spawn((MyPanel, ChildOf(body)));
    }

    let mut app = test_app();
    app.add_systems(Update, spawn_panel);
    declare(&mut app, "audio", "Audio", 10);
    app.update();
    app.update();
    app.update();

    let body = body_of(&mut app, "audio").unwrap();
    let mut q = app.world_mut().query_filtered::<&ChildOf, With<MyPanel>>();
    let parents: Vec<Entity> = q.iter(app.world()).map(ChildOf::parent).collect();
    assert_eq!(parents, [body]);
}

#[test]
fn a_condition_on_a_missing_tab_never_opens() {
    #[derive(Resource, Default)]
    struct Ran(u32);

    fn count(mut ran: ResMut<Ran>) {
        ran.0 += 1;
    }

    let mut app = test_app();
    app.init_resource::<Ran>()
        .add_systems(Update, count.run_if(tab_exists("nonexistent")));
    declare(&mut app, "general", "General", 10);
    app.update();
    app.update();

    assert_eq!(app.world().resource::<Ran>().0, 0);
}

#[test]
fn open_mirrors_onto_visibility_and_is_never_set_here() {
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    app.update();
    app.update();

    let root = dialog(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(root),
        Some(&Visibility::Hidden),
        "a freshly built dialog starts closed"
    );

    app.world_mut().get_mut::<SettingsOpen>(root).unwrap().0 = true;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(root),
        Some(&Visibility::Inherited)
    );

    // Several frames with nothing touching it: the crate must not decide
    // to open or close on its own.
    for _ in 0..3 {
        app.update();
    }
    assert!(app.world().get::<SettingsOpen>(root).unwrap().0);
}

#[test]
fn two_dialogs_do_not_share_a_selection() {
    // The reason `ActiveTab` is a component and not a resource.
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    declare(&mut app, "audio", "Audio", 20);
    app.update();
    app.update();

    let first = dialog(&mut app);
    // A second dialog, as a host with a per-project settings window builds.
    let second = app
        .world_mut()
        .spawn((
            SettingsDialog,
            ActiveTab::at("general"),
            SettingsOpen(false),
        ))
        .id();

    app.world_mut()
        .get_mut::<ActiveTab>(first)
        .unwrap()
        .set("audio");
    app.update();

    assert!(app.world().get::<ActiveTab>(first).unwrap().is("audio"));
    assert!(
        app.world().get::<ActiveTab>(second).unwrap().is("general"),
        "the second dialog keeps its own selection"
    );
}

#[test]
fn the_body_scrolls_only_while_open() {
    let mut app = test_app();
    declare(&mut app, "general", "General", 10);
    app.update();
    app.update();

    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<SettingsBody>>();
    let body = q.iter(app.world()).next().expect("a body exists");

    // Closed: a wheel message must not move it.
    scroll_down(&mut app);
    app.update();
    assert_eq!(app.world().get::<ScrollPosition>(body).unwrap().0.y, 0.0);

    let root = dialog(&mut app);
    app.world_mut().get_mut::<SettingsOpen>(root).unwrap().0 = true;
    scroll_down(&mut app);
    app.update();
    assert!(
        app.world().get::<ScrollPosition>(body).unwrap().0.y > 0.0,
        "an open dialog scrolls"
    );
}
