//! Right-click to open a menu built from *declared* items rather than
//! from a `Vec` assembled at the call site.
//!
//! Three things this shows that the `context_menu` example cannot:
//!
//! - items are spawned by whoever owns the feature, in any order;
//! - `VisibleWhen` gates them on live state (toggle it with Space);
//! - a submenu is just `ChildOf`, and activation comes back as one
//!   message carrying the entity that declared the row.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tempera::context_menu::{
    AppMenuExt, Destructive, MenuClosed, MenuItemActivated, MenuLabel, MenuOrder, SeparatorBefore,
    VisibleWhen, child_item, menu_item, open_surface,
};
use tempera::prelude::*;

const SURFACE: &str = "demo.canvas";

/// Stand-in for whatever the host's selection actually is.
#[derive(Resource, Default)]
struct HasSelection(bool);

fn has_selection(s: Res<HasSelection>) -> bool {
    s.0
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .init_resource::<HasSelection>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                open_on_right_click,
                toggle_selection,
                log_activation,
                log_close,
                update_banner,
            ),
        );

    // Declared once, at startup. In a real app each of these lives in
    // the crate that owns the feature, and none of them knows about the
    // others.
    app.spawn_menu_item((menu_item(SURFACE, "Rename"), MenuOrder(10)));
    app.spawn_menu_item((
        menu_item(SURFACE, "Duplicate"),
        MenuOrder(20),
        VisibleWhen::new(has_selection),
    ));
    app.spawn_menu_item((
        menu_item(SURFACE, "Delete"),
        MenuOrder(90),
        SeparatorBefore,
        Destructive,
        VisibleWhen::new(has_selection),
    ));

    // A submenu: the parent is an ordinary item, the children are
    // ordinary items with `ChildOf`.
    let parent = app.world_mut().spawn(menu_item(SURFACE, "Add")).id();
    for (i, shape) in ["Sine", "Triangle", "Square"].iter().enumerate() {
        app.world_mut()
            .spawn((child_item(*shape), MenuOrder(i as i32), ChildOf(parent)));
    }
    app.world_mut()
        .entity_mut(parent)
        .insert((MenuOrder(30), Name::new("menu:Add")));

    app.run();
}

#[derive(Component)]
struct Banner;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);

    let inter: Handle<Font> = asset_server.load("fonts/Inter-Regular.otf");
    font.regular = Some(inter.clone());

    commands.spawn((
        Banner,
        Text::new(""),
        TextFont {
            font: FontSource::Handle(inter),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
        },
    ));
}

fn update_banner(
    selection: Res<HasSelection>,
    mut banner: Query<&mut Text, With<Banner>>,
) -> Result {
    if !selection.is_changed() {
        return Ok(());
    }
    let state = if selection.0 { "yes" } else { "no" };
    banner.single_mut()?.0 =
        format!("Right-click anywhere.\nSpace toggles selection (currently: {state})");
    Ok(())
}

fn toggle_selection(keys: Res<ButtonInput<KeyCode>>, mut selection: ResMut<HasSelection>) {
    if keys.just_pressed(KeyCode::Space) {
        selection.0 = !selection.0;
    }
}

/// The registry needs `&mut World`, so opening is a one-shot command
/// rather than a message write.
fn open_on_right_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    commands.queue(move |world: &mut World| {
        open_surface(world, SURFACE, cursor);
    });
}

/// Activation is *reported*, not dispatched: the host decides what a
/// click means. `entity` is the item that declared the row, so behaviour
/// can hang off it as components rather than off a string registry.
fn log_activation(mut reader: MessageReader<MenuItemActivated>, labels: Query<&MenuLabel>) {
    for ev in reader.read() {
        let label = ev
            .entity
            .and_then(|e| labels.get(e).ok())
            .map(|l| l.as_str())
            .unwrap_or("<unknown>");
        info!("activated {label:?} (declared by {:?})", ev.entity);
    }
}

/// One place to clear whatever the host stashed when the menu opened —
/// which row was right-clicked, say — however the menu went away.
fn log_close(mut reader: MessageReader<MenuClosed>) {
    for _ in reader.read() {
        info!("menu closed");
    }
}
