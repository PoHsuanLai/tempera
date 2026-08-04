//! Separator + Progress + Kbd in one panel. The progress bar animates
//! 0→1→0 in a loop so you can confirm the fill repaint.

use bevy::prelude::*;
use tempera::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_progress)
        .run();
}

#[derive(Component)]
struct Ticker(f32);

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
    sep_style: SeparatorStyle,
    prog_style: ProgressStyle,
    kbd_style: KbdStyle,
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
        Text::new("Separator (horizontal, 320px)".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));
    let sep = spawn_separator(
        &mut commands,
        &sep_style,
        SeparatorAxis::Horizontal,
        Some(320.0),
    );
    commands.entity(sep).insert(ChildOf(root));

    commands.spawn((
        Text::new("Progress (animating)".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));
    let prog = spawn_progress(&mut commands, &prog_style, 320.0, 0.0);
    commands
        .entity(prog)
        .insert(ChildOf(root))
        .insert(Ticker(0.0));

    commands.spawn((
        Text::new("Kbd: \"Ctrl+Shift+P\"".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));
    let kbd = spawn_kbd(&mut commands, &kbd_style, "Ctrl+Shift+P");
    commands.entity(kbd).insert(ChildOf(root));
}

fn animate_progress(time: Res<Time>, mut q: Query<(&mut ProgressValue, &mut Ticker)>) {
    for (mut value, mut ticker) in &mut q {
        ticker.0 = (ticker.0 + time.delta_secs() * 0.5) % 2.0;
        let t = if ticker.0 <= 1.0 {
            ticker.0
        } else {
            2.0 - ticker.0
        };
        value.0 = t;
    }
}
