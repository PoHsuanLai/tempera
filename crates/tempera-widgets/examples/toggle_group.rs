//! ToggleGroup demo. One Single group (radio-style) and one Multiple
//! group (checkbox-style). Click items or focus + arrow keys.

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
    font.regular = Some(inter);

    commands.queue(|world: &mut World| {
        world.run_system_cached(spawn_grid).ok();
    });
}

fn spawn_grid(
    mut commands: Commands,
    style: ToggleGroupStyle,
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
        ))
        .id();

    let label_font = font.text_font(typography.sm);

    commands.spawn((
        Text::new("Single (exclusive)".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    let single = spawn_toggle_group(
        &mut commands,
        &style,
        ToggleGroupKind::Single,
        vec![
            ToggleGroupItem::new("LEFT"),
            ToggleGroupItem::new("CENTER").selected(),
            ToggleGroupItem::new("RIGHT"),
        ],
    );
    commands
        .entity(single)
        .insert(ChildOf(root))
        .observe(|on: On<ValueChange<Entity>>| info!("single -> {:?}", on.value));

    commands.spawn((
        Text::new("Multiple (independent)".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));

    let multi = spawn_toggle_group(
        &mut commands,
        &style,
        ToggleGroupKind::Multiple,
        vec![
            ToggleGroupItem::new("Bold"),
            ToggleGroupItem::new("Italic"),
            ToggleGroupItem::new("Underline"),
        ],
    );
    commands.entity(multi).insert(ChildOf(root));
    // For Multiple, observe per-item — not on the group root.
    // Skipped in this example to keep code small.
}
