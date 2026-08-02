//! An installed-extensions list built from `list_row`s, showing the thing
//! a form row cannot do: **two controls in one trailing slot**.
//!
//! Each row carries a `ListRowId`, so the toggle observer names its record
//! by id rather than by entity — which is what lets a list be re-emitted
//! (filtered, sorted, rescanned) without the observers going stale.
//!
//! Pass `--screenshot <path>` to capture frame 120 and exit.

use std::collections::HashSet;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui_widgets::ValueChange;
use tempera::prelude::*;

/// One record the list is built from. A real host would scan for these.
struct Entry {
    id: &'static str,
    name: &'static str,
    version: &'static str,
    kind: &'static str,
    description: &'static str,
}

const ENTRIES: &[Entry] = &[
    Entry {
        id: "com.acme.reverb",
        name: "Plate Reverb",
        version: "1.2.0",
        kind: "Audio Effect",
        description: "A plate reverb with modulation and a long, deliberately overlong description that has to truncate",
    },
    Entry {
        id: "com.acme.piano",
        name: "Felt Piano",
        version: "0.4.1",
        kind: "Instrument",
        description: "Sampled upright with the felt down",
    },
    Entry {
        id: "org.example.日本語",
        name: "日本語アダプタ",
        version: "2.0.0",
        kind: "Language",
        description: "多バイト文字の説明文です。バイト単位で切ると壊れます",
    },
];

/// Which extensions are enabled. The host's state, not the widget's.
#[derive(Resource, Default)]
struct Enabled(HashSet<&'static str>);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .insert_resource(Enabled(
            ENTRIES.iter().map(|e| e.id).collect::<HashSet<_>>(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, build_once);
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

#[derive(Component)]
struct RowList;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);
    font.regular = Some(asset_server.load("fonts/Inter-Regular.otf"));

    commands.spawn((
        RowList,
        Node {
            width: Val::Px(460.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::all(Val::Px(24.0)),
            ..default()
        },
    ));
}

fn build_once(
    mut commands: Commands,
    style: ListRowStyle,
    switch_style: tempera::switch::SwitchStyle,
    enabled: Res<Enabled>,
    list: Query<Entity, With<RowList>>,
    existing: Query<(), With<ListRow>>,
) {
    if !existing.is_empty() || style.font.regular.is_none() {
        return;
    }
    let Ok(list) = list.single() else { return };

    for entry in ENTRIES {
        let parts = spawn_list_row(
            &mut commands,
            &style,
            ListRowSpec::new(entry.id, entry.name)
                .meta(format!("v{}", entry.version))
                .badge(entry.kind)
                .subtitle(entry.description),
        );
        commands.entity(parts.row).insert(ChildOf(list));

        // Two widgets in one trailing slot — a form row holds one.
        let sw = spawn_switch(&mut commands, &switch_style, enabled.0.contains(entry.id));
        let id = entry.id;
        commands.entity(sw).insert(ChildOf(parts.trail)).observe(
            move |on: On<ValueChange<bool>>, mut en: ResMut<Enabled>| {
                if on.value {
                    en.0.insert(id);
                } else {
                    en.0.remove(id);
                }
            },
        );
    }
}

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
