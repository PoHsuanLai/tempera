//! Tabs row + a dropdown trigger. Click a tab to slide the indicator;
//! click the "File" button to open a dropdown menu.
//!
//! Pass `--screenshot <path>` (e.g. `--screenshot /tmp/tabs.png`) to
//! capture frame 120 and exit. Used during widget iteration so we
//! don't have to drive the UI by hand.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use tempera::context_menu::MenuItemActivated;
use tempera::prelude::*;

fn parse_screenshot_arg() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
}

fn open_dropdown_flag() -> bool {
    std::env::args().any(|a| a == "--open-dropdown")
}

/// Programmatically open the File dropdown one frame after spawn so
/// the screenshot captures the popup. Bypasses the trigger to keep
/// keyboard focus from grabbing the dropdown's first item.
fn auto_open_dropdown(
    mut writer: MessageWriter<OpenContextMenu>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    writer.write(OpenContextMenu(MenuRequest {
        // Tuck the menu under where the "File" button renders.
        // Approximate; tweak if the screenshot misframes.
        anchor: Vec2::new(40.0, 200.0),
        items: vec![
            MenuItemSpec::new("new").label("New File").shortcut("⌘N"),
            MenuItemSpec::new("open").label("Open…").shortcut("⌘O"),
            MenuItemSpec::new("save").label("Save").shortcut("⌘S"),
            MenuItemSpec::new("quit")
                .label("Quit")
                .shortcut("⌘Q")
                .destructive()
                .separator_before(),
        ],
    }));
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
        info!(
            "[screenshot] capturing at frame {} -> {}",
            req.frames,
            path.display()
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    // Wait a few frames so the async write lands before we exit.
    if req.captured && req.frames > req.target + 30 {
        exit.write(AppExit::Success);
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, log_menu_clicks);
    if let Some(path) = parse_screenshot_arg() {
        app.insert_resource(ScreenshotRequest {
            path,
            frames: 0,
            target: 120,
            captured: false,
        })
        .add_systems(Update, auto_screenshot);
    }
    if open_dropdown_flag() {
        app.add_systems(Update, auto_open_dropdown);
    }
    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);
    let inter: Handle<Font> = asset_server.load("fonts/Inter-Regular.otf");
    font.0 = Some(inter);

    commands.queue(|world: &mut World| {
        world.run_system_cached(spawn_grid).ok();
    });
}

fn spawn_grid(
    mut commands: Commands,
    tabs_style: TabsStyle,
    dropdown_style: DropdownStyle,
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
        Text::new("Tabs (animated indicator)".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    let tabs = spawn_tabs(
        &mut commands,
        &tabs_style,
        vec!["Files".into(), "Search".into(), "Git".into(), "Debug".into()],
        0,
    );
    commands
        .entity(tabs)
        .insert(ChildOf(root))
        .observe(|on: On<TabsChanged>| info!("tab -> {}", on.active));

    commands.spawn((
        Text::new("Dropdown (\"File\")".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    let dd = spawn_dropdown(
        &mut commands,
        &dropdown_style,
        "File",
        vec![
            MenuItemSpec::new("new").label("New File").shortcut("⌘N"),
            MenuItemSpec::new("open").label("Open…").shortcut("⌘O"),
            MenuItemSpec::new("save").label("Save").shortcut("⌘S"),
            MenuItemSpec::new("quit")
                .label("Quit")
                .shortcut("⌘Q")
                .destructive()
                .separator_before(),
        ],
    );
    commands.entity(dd).insert(ChildOf(root));
}

fn log_menu_clicks(mut reader: MessageReader<MenuItemActivated>) {
    for ev in reader.read() {
        info!("menu activated: {}", ev.id);
    }
}
