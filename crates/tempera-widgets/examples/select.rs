//! Select widget demo.
//!
//! `--screenshot <path>` captures frame 120 and exits.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use tempera::prelude::*;
use tempera::select::{spawn_select, SelectOption, SelectStyle, ValueChange as SelectValueChange};

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
        });
        app.add_systems(Update, auto_screenshot);
    }

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);
    let inter: Handle<Font> = asset_server.load("fonts/Inter-Regular.otf");
    font.regular = Some(inter);

    commands.queue(|world: &mut World| {
        world.run_system_cached(spawn_demo).ok();
    });
}

fn spawn_demo(
    mut commands: Commands,
    select_style: SelectStyle,
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

    // Label
    commands.spawn((
        Text::new("Output Routing"),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    // Select — routing destinations
    let options = vec![
        SelectOption { id: "master".into(), label: "Master".into() },
        SelectOption { id: "bus_a".into(), label: "Bus A".into() },
        SelectOption { id: "bus_b".into(), label: "Bus B".into() },
        SelectOption { id: "headphones".into(), label: "Headphones".into() },
    ];
    let sel = spawn_select(&mut commands, &select_style, options, "master");
    commands.entity(sel).insert(ChildOf(root));
    commands.entity(sel).observe(
        |trigger: On<SelectValueChange>| {
            let ev = trigger.event();
            info!("selected: {} ({})", ev.label, ev.value);
        },
    );

    // Second label
    commands.spawn((
        Text::new("Input Source"),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    // Select — input sources
    let inputs = vec![
        SelectOption { id: "none".into(), label: "None".into() },
        SelectOption { id: "mic_1".into(), label: "Mic 1".into() },
        SelectOption { id: "mic_2".into(), label: "Mic 2".into() },
        SelectOption { id: "stereo_1_2".into(), label: "Stereo 1/2".into() },
        SelectOption { id: "midi_keyboard".into(), label: "MIDI Keyboard".into() },
    ];
    let sel2 = spawn_select(&mut commands, &select_style, inputs, "none");
    commands.entity(sel2).insert(ChildOf(root));
    commands.entity(sel2).observe(
        |trigger: On<SelectValueChange>| {
            let ev = trigger.event();
            info!("input: {} ({})", ev.label, ev.value);
        },
    );

    // Third — wider select
    commands.spawn((
        Text::new("Sample Rate"),
        label_font,
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    let rates = vec![
        SelectOption { id: "44100".into(), label: "44.1 kHz".into() },
        SelectOption { id: "48000".into(), label: "48 kHz".into() },
        SelectOption { id: "96000".into(), label: "96 kHz".into() },
        SelectOption { id: "192000".into(), label: "192 kHz".into() },
    ];
    let sel3 = spawn_select(&mut commands, &select_style, rates, "48000");
    commands.entity(sel3).insert(ChildOf(root));
    commands.entity(sel3).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Px(160.0);
    });
}
