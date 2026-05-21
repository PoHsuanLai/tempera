//! Checkbox + Switch demo. Click or focus + Space/Enter to flip.

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
    checkbox_style: CheckboxStyle,
    switch_style: SwitchStyle,
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

    let label = |parent: &mut ChildSpawnerCommands, text: &str| {
        parent.spawn((
            Text::new(text.to_string()),
            label_font.clone(),
            TextColor(palette.muted_foreground),
        ));
    };

    // Checkboxes
    commands
        .entity(root)
        .with_children(|p| label(p, "Checkboxes (unchecked, checked, disabled)"));
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    let cb1 = spawn_checkbox(&mut commands, &checkbox_style, false);
    let cb2 = spawn_checkbox(&mut commands, &checkbox_style, true);
    let cb3 = spawn_checkbox(&mut commands, &checkbox_style, true);
    for id in [cb1, cb2] {
        commands
            .entity(id)
            .insert(ChildOf(row))
            .observe(|on: On<ValueChange<bool>>| info!("checkbox -> {}", on.value));
    }
    commands
        .entity(cb3)
        .insert(ChildOf(row))
        .insert(bevy::ui::InteractionDisabled);

    // Switches
    commands
        .entity(root)
        .with_children(|p| label(p, "Switches (off, on, disabled)"));
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    let s1 = spawn_switch(&mut commands, &switch_style, false);
    let s2 = spawn_switch(&mut commands, &switch_style, true);
    let s3 = spawn_switch(&mut commands, &switch_style, true);
    for id in [s1, s2] {
        commands
            .entity(id)
            .insert(ChildOf(row))
            .observe(|on: On<ValueChange<bool>>| info!("switch -> {}", on.value));
    }
    commands
        .entity(s3)
        .insert(ChildOf(row))
        .insert(bevy::ui::InteractionDisabled);
}
