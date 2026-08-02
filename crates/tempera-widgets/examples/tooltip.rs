//! Tooltip demo. Hover any button to reveal a contextual hint.
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
    font.regular = Some(inter);

    commands.queue(|world: &mut World| {
        world.run_system_cached(spawn_grid).ok();
    });
}

fn spawn_grid(
    mut commands: Commands,
    button_style: ButtonStyle,
    font: Res<FontHandle>,
    typography: Res<Typography>,
    palette: Res<ColorPalette>,
) {
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(40.0)),
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(palette.background),
        ))
        .id();

    let label_font = font.text_font(typography.sm);

    commands.spawn((
        Text::new("Hover the buttons to see tooltips.".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(20.0),
                margin: UiRect::top(Val::Px(40.0)),
                ..default()
            },
            ChildOf(root),
        ))
        .id();

    let save = spawn_button(
        &mut commands,
        &button_style,
        ButtonContent::text("Save"),
        ButtonVariant::Default,
    );
    commands.entity(save).insert(ChildOf(row)).insert(
        Tooltip::new("Save Changes")
            .shortcut(KbdChord::from(ModifierKey::Super).with(KeyCode::KeyS)),
    );

    let undo = spawn_button(
        &mut commands,
        &button_style,
        ButtonContent::text("Undo"),
        ButtonVariant::Outline,
    );
    commands.entity(undo).insert(ChildOf(row)).insert(
        Tooltip::new("Undo")
            .shortcut(KbdChord::from(ModifierKey::Super).with(KeyCode::KeyZ))
            .position(TooltipPosition::Bottom)
            .delay(250),
    );

    let danger = spawn_button(
        &mut commands,
        &button_style,
        ButtonContent::text("Delete"),
        ButtonVariant::Destructive,
    );
    commands.entity(danger).insert(ChildOf(row)).insert(
        Tooltip::new("Delete the selected track permanently")
            .max_width(260.0)
            .shortcut(KbdChord::from(ModifierKey::Shift).with(KeyCode::Delete))
            .position(TooltipPosition::Top),
    );
}
