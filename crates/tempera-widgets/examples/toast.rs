//! Toast demo. Click the buttons to fire toasts of different
//! variants. The "Export…" button enqueues an external-progress
//! toast and ticks it forward over several seconds, then switches
//! to auto-dismiss.
//!
//! `--screenshot <path>` captures frame 120 and exits.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use tempera::button::Activate;
use tempera::prelude::*;

fn parse_screenshot_arg() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
}

/// Auto-fire a few toasts at startup so the screenshot captures the
/// stack without needing manual clicks.
fn open_toasts_flag() -> bool {
    std::env::args().any(|a| a == "--open-toasts")
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

#[derive(Component)]
struct ToastKind(Kind);

#[derive(Clone, Copy)]
enum Kind {
    Default,
    Error,
    WithTitle,
    LongProgress,
}

/// Tracks the in-flight progress toast so we can update it each frame.
#[derive(Resource, Default)]
struct ProgressToast {
    id: Option<ToastId>,
    elapsed: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .init_resource::<ProgressToast>()
        .add_systems(Startup, setup)
        .add_systems(Update, (drive_progress_toast, maybe_auto_fire));
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
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(palette.background),
        ))
        .id();

    let label_font = font.text_font(typography.sm);

    commands.spawn((
        Text::new("Click a button to fire a toast.".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();

    spawn_kind_button(&mut commands, &button_style, row, "Default", Kind::Default);
    spawn_kind_button(
        &mut commands,
        &button_style,
        row,
        "With title",
        Kind::WithTitle,
    );
    spawn_kind_button(&mut commands, &button_style, row, "Error", Kind::Error);
    spawn_kind_button(
        &mut commands,
        &button_style,
        row,
        "Export…",
        Kind::LongProgress,
    );
}

fn spawn_kind_button(
    commands: &mut Commands,
    style: &ButtonStyle,
    parent: Entity,
    label: &str,
    kind: Kind,
) {
    let variant = match kind {
        Kind::Error => ButtonVariant::Destructive,
        Kind::LongProgress => ButtonVariant::Outline,
        _ => ButtonVariant::Default,
    };
    let id = spawn_button(commands, style, ButtonContent::text(label), variant);
    commands
        .entity(id)
        .insert(ChildOf(parent))
        .insert(ToastKind(kind))
        .observe(fire_toast);
}

fn fire_toast(
    on: On<Activate>,
    kinds: Query<&ToastKind>,
    mut toasts: ResMut<ToastManager>,
    mut progress: ResMut<ProgressToast>,
) {
    let Ok(kind) = kinds.get(on.entity) else {
        return;
    };
    match kind.0 {
        Kind::Default => {
            toasts.toast("Changes saved");
        }
        Kind::Error => {
            toasts.error("Failed to render: out of memory");
        }
        Kind::WithTitle => {
            toasts
                .custom()
                .title("Scheduled")
                .message("Your message has been queued for delivery.")
                .duration(Duration::from_secs(6))
                .show();
        }
        Kind::LongProgress => {
            // Replace any in-flight progress toast.
            if let Some(id) = progress.id.take() {
                toasts.dismiss(id);
            }
            let id = toasts
                .custom()
                .title("Exporting")
                .message("Rendering audio…")
                .progress(0.0)
                .show();
            progress.id = Some(id);
            progress.elapsed = 0.0;
        }
    }
}

/// Tick the in-flight progress toast forward over 4 seconds, then
/// flip to the timed-countdown ("Done!") for another moment.
fn drive_progress_toast(
    time: Res<Time>,
    mut toasts: ResMut<ToastManager>,
    mut state: ResMut<ProgressToast>,
) {
    let Some(id) = state.id else {
        return;
    };
    state.elapsed += time.delta_secs();
    let duration = 4.0;
    if state.elapsed < duration {
        let p = state.elapsed / duration;
        toasts.set_progress(id, p);
        // Update message at the midway point so the demo shows
        // `set_message` in action.
        if state.elapsed > duration * 0.5 {
            toasts.set_message(id, "Encoding…");
        }
    } else {
        toasts.set_message(id, "Export complete!");
        toasts.start_dismiss(id);
        state.id = None;
    }
}

fn maybe_auto_fire(mut done: Local<bool>, mut toasts: ResMut<ToastManager>) {
    if *done || !open_toasts_flag() {
        return;
    }
    *done = true;
    toasts.toast("Project saved");
    toasts
        .custom()
        .title("Scheduled")
        .message("Your message has been queued for delivery.")
        .duration(Duration::from_secs(6))
        .show();
    toasts.error("Failed to render: out of memory");
}
