//! One row per `ButtonVariant`, each variant in all four `ButtonSize`s.
//! Click any button to log via `On<Activate>`.

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

    // Defer to a one-shot system so we get `ButtonStyle` resolved with
    // populated palette/spacing/font.
    commands.queue(|world: &mut World| {
        world.run_system_cached(spawn_grid).ok();
    });
}

fn spawn_grid(
    mut commands: Commands,
    style: ButtonStyle,
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
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(palette.background),
            Name::new("button-example-root"),
        ))
        .id();

    let label_font = font.text_font(typography.xs);

    for variant in [
        ButtonVariant::Default,
        ButtonVariant::Secondary,
        ButtonVariant::Outline,
        ButtonVariant::Ghost,
        ButtonVariant::Link,
        ButtonVariant::Destructive,
    ] {
        let row = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(root),
            ))
            .id();

        commands.spawn((
            Text::new(format!("{variant:?}")),
            label_font.clone(),
            TextColor(palette.muted_foreground),
            Node {
                width: Val::Px(110.0),
                ..default()
            },
            ChildOf(row),
        ));

        for size in [
            ButtonSize::Xs,
            ButtonSize::Sm,
            ButtonSize::Md,
            ButtonSize::Lg,
        ] {
            let id = spawn_button(
                &mut commands,
                &style,
                ButtonContent::text(format!("{size:?}")),
                variant,
            );
            commands.entity(id).insert(size).insert(ChildOf(row));
            let label = format!("{variant:?}-{size:?}");
            commands
                .entity(id)
                .observe(move |_: On<Activate>| info!("clicked: {label}"));
        }
    }

    // Last row: a disabled button to verify the dim state.
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        Text::new("Disabled"),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        Node {
            width: Val::Px(110.0),
            ..default()
        },
        ChildOf(row),
    ));
    let id = spawn_button(
        &mut commands,
        &style,
        ButtonContent::text("Can't click"),
        ButtonVariant::Default,
    );
    commands
        .entity(id)
        .insert(bevy::ui::InteractionDisabled)
        .insert(ChildOf(row));
}
