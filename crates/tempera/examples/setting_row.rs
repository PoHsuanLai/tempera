//! A settings form built from `setting_row`s, with a `list_row` section
//! beneath it so the two are visible side by side.
//!
//! The point of the pairing: the form's controls line up in a column
//! because `setting_row`'s slot is fixed-width, while the list's trailing
//! content is ragged because it holds a different number of widgets per
//! row. Neither is a mode of the other.
//!
//! Pass `--screenshot <path>` to capture frame 120 and exit.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui_widgets::ValueChange;
use tempera::prelude::*;
use tempera::slider::{SliderRange, SliderValue};

/// The host's settings. A real app would persist these.
#[derive(Resource)]
struct Prefs {
    auto_save: bool,
    font_size: f32,
    show_fps: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            auto_save: true,
            font_size: 14.0,
            show_fps: false,
        }
    }
}

#[derive(Component)]
struct Form;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .init_resource::<Prefs>()
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);
    font.regular = Some(asset_server.load("fonts/Inter-Regular.otf"));

    commands.spawn((
        Form,
        Node {
            width: Val::Px(560.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::all(Val::Px(24.0)),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn build_once(
    mut commands: Commands,
    style: SettingRowStyle,
    list_style: ListRowStyle,
    switch_style: tempera::switch::SwitchStyle,
    slider_style: tempera::slider::SliderStyle,
    prefs: Res<Prefs>,
    form: Query<Entity, With<Form>>,
    existing: Query<(), With<SettingRow>>,
) {
    if !existing.is_empty() || style.font.regular.is_none() {
        return;
    }
    let Ok(form) = form.single() else { return };

    // ── A form: labelled controls, aligned down a fixed-width column ──
    spawn_section_header(&mut commands, &style, form, "GENERAL");

    let slot = spawn_setting_row(
        &mut commands,
        &style,
        form,
        SettingRowSpec::new("Auto Save").description("Save the project every 5 minutes"),
    );
    let sw = spawn_switch(&mut commands, &switch_style, prefs.auto_save);
    commands.entity(sw).insert(ChildOf(slot)).observe(
        |on: On<ValueChange<bool>>, mut prefs: ResMut<Prefs>| {
            prefs.auto_save = on.value;
        },
    );

    spawn_section_header(&mut commands, &style, form, "APPEARANCE");

    let slot = spawn_setting_row(
        &mut commands,
        &style,
        form,
        SettingRowSpec::new("Font Size").description("Base font size for the interface"),
    );
    let slider = spawn_slider(
        &mut commands,
        &slider_style,
        SliderRange::new(10.0, 20.0),
        SliderValue(prefs.font_size),
    );
    commands.entity(slider).insert(ChildOf(slot)).observe(
        |on: On<ValueChange<f32>>, mut prefs: ResMut<Prefs>| {
            prefs.font_size = on.value;
        },
    );

    let slot = spawn_setting_row(&mut commands, &style, form, SettingRowSpec::new("Show FPS"));
    let sw = spawn_switch(&mut commands, &switch_style, prefs.show_fps);
    commands.entity(sw).insert(ChildOf(slot)).observe(
        |on: On<ValueChange<bool>>, mut prefs: ResMut<Prefs>| {
            prefs.show_fps = on.value;
        },
    );

    // ── A list: discovered records, ragged trailing content, hover ──
    spawn_section_header(&mut commands, &style, form, "INSTALLED");

    for (id, name, version, kind) in [
        ("com.acme.reverb", "Plate Reverb", "1.2.0", "Audio Effect"),
        ("com.acme.piano", "Felt Piano", "0.4.1", "Instrument"),
    ] {
        let parts = spawn_list_row(
            &mut commands,
            &list_style,
            ListRowSpec::new(id, name)
                .meta(format!("v{version}"))
                .badge(kind)
                .subtitle("Hover me — a form row above does not tint"),
        );
        commands.entity(parts.row).insert(ChildOf(form));
        let sw = spawn_switch(&mut commands, &switch_style, true);
        commands.entity(sw).insert(ChildOf(parts.trail));
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
