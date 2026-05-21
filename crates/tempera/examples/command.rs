//! Command-palette demo. The palette is spawned centered on the
//! window. Type to filter, Up/Down to move selection, Enter to
//! activate, click to activate.
//!
//! `--screenshot <path>` captures frame 120 and exits.

use std::path::PathBuf;

use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use leafwing_input_manager::user_input::keyboard::ModifierKey;
use tempera::kbd::KbdChord;
use tempera::prelude::*;

fn parse_screenshot_arg() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
}

#[derive(Resource)]
struct ScreenshotRequest {
    path: PathBuf,
    frames: u32,
    target: u32,
    captured: bool,
}

fn auto_screenshot(
    mut commands: Commands,
    mut req: ResMut<ScreenshotRequest>,
    mut exit: MessageWriter<AppExit>,
) {
    req.frames += 1;
    if req.frames == req.target && !req.captured {
        req.captured = true;
        let path = req.path.clone();
        info!("[screenshot] capturing -> {}", path.display());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if req.captured && req.frames > req.target + 30 {
        exit.write(AppExit::Success);
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .add_systems(Startup, setup);
    if let Some(path) = parse_screenshot_arg() {
        app.insert_resource(ScreenshotRequest {
            path,
            frames: 0,
            target: 120,
            captured: false,
        })
        .add_systems(Update, auto_screenshot);
    }
    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);
    let inter: Handle<Font> = asset_server.load("fonts/Inter-Regular.otf");
    font.0 = Some(inter);

    commands.queue(|world: &mut World| {
        world.run_system_cached(spawn_palette).ok();
    });
}

fn spawn_palette(
    mut commands: Commands,
    command_style: CommandStyle,
    palette: Res<ColorPalette>,
) {
    let backdrop = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                padding: UiRect::top(Val::Px(120.0)),
                ..default()
            },
            BackgroundColor(palette.background),
        ))
        .id();

    let cmd = spawn_command(
        &mut commands,
        &command_style,
        "Type a command or search…",
        vec![
            CommandSection::new("Suggestions")
                .item(
                    CommandItemSpec::new("calendar")
                        .label("Calendar")
                        .shortcut(KbdChord::from(ModifierKey::Super).with(KeyCode::KeyC)),
                )
                .item(
                    CommandItemSpec::new("emoji")
                        .label("Search Emoji"),
                )
                .item(
                    CommandItemSpec::new("calculator")
                        .label("Calculator")
                        .disabled(),
                ),
            CommandSection::new("Settings")
                .item(
                    CommandItemSpec::new("profile")
                        .label("Profile")
                        .shortcut(KbdChord::from(ModifierKey::Super).with(KeyCode::KeyP)),
                )
                .item(
                    CommandItemSpec::new("billing")
                        .label("Billing")
                        .shortcut(KbdChord::from(ModifierKey::Super).with(KeyCode::KeyB)),
                )
                .item(
                    CommandItemSpec::new("settings")
                        .label("Settings")
                        .shortcut(KbdChord::from(ModifierKey::Super).with(KeyCode::KeyS)),
                ),
        ],
    );
    commands
        .entity(cmd)
        .insert(ChildOf(backdrop))
        .observe(|on: On<CommandActivated>| info!("activated: {}", on.id));
}
