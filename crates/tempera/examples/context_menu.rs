//! Right-click anywhere in the window to open a tempera context menu.
//! Click an item, click outside, or press Escape to close.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tempera::context_menu::MenuItemActivated;
use tempera::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (open_on_right_click, log_activation))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);

    // Use Inter so menu shortcut glyphs (⌘, ⌫, arrows) render correctly.
    // Tempera consumers point FontHandle at whatever font fits their app —
    // here we reuse the copy that ships with dawai-theme.
    let inter: Handle<Font> = asset_server.load("fonts/Inter-Regular.otf");
    font.0 = Some(inter.clone());

    commands.spawn((
        Text::new("Right-click anywhere"),
        TextFont {
            font: inter,
            font_size: 18.0,
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

fn open_on_right_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut writer: MessageWriter<OpenContextMenu>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(pos) = window.cursor_position() else {
        return;
    };
    writer.write(OpenContextMenu(MenuRequest {
        anchor: pos,
        items: vec![
            MenuItemSpec::new("rename").label("Rename"),
            MenuItemSpec::new("duplicate")
                .label("Duplicate")
                .shortcut("⌘D"),
            MenuItemSpec::new("export")
                .label("Export…")
                .shortcut("⌘E"),
            MenuItemSpec::new("locked").label("Locked Item").disabled(),
            MenuItemSpec::new("delete")
                .label("Delete")
                .shortcut("⌫")
                .destructive()
                .separator_before(),
        ],
    }));
}

fn log_activation(mut reader: MessageReader<MenuItemActivated>) {
    for ev in reader.read() {
        info!("activated: {}", ev.id);
    }
}
