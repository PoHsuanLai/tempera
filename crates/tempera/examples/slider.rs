//! Three sliders across different ranges. Drag the thumb or use
//! arrow keys when focused. `ValueChange<f32>` is observed and logged.

use bevy::prelude::*;
use tempera::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .add_systems(Startup, setup)
        .run();
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
    style: SliderStyle,
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
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(palette.background),
            Name::new("slider-example-root"),
        ))
        .id();

    let label_font = font.text_font(typography.sm);

    let configs: [(&str, SliderRange, SliderValue, SliderSize); 3] = [
        (
            "Volume 0..1 (Sm)",
            SliderRange::new(0.0, 1.0),
            SliderValue(0.65),
            SliderSize::Sm,
        ),
        (
            "Pan -1..1 (Md)",
            SliderRange::new(-1.0, 1.0),
            SliderValue(0.0),
            SliderSize::Md,
        ),
        (
            "BPM 60..240 (Lg)",
            SliderRange::new(60.0, 240.0),
            SliderValue(120.0),
            SliderSize::Lg,
        ),
    ];

    for (label, range, value, size) in configs {
        let row = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    width: Val::Px(360.0),
                    ..default()
                },
                ChildOf(root),
            ))
            .id();

        commands.spawn((
            Text::new(label.to_string()),
            label_font.clone(),
            TextColor(palette.muted_foreground),
            ChildOf(row),
        ));

        let id = spawn_slider(&mut commands, &style, range, value);
        commands.entity(id).insert(size).insert(ChildOf(row));
        let owned = label.to_string();
        commands
            .entity(id)
            .observe(move |on: On<ValueChange<f32>>| {
                info!("{owned}: {:.3}", on.value);
            });
    }
}
