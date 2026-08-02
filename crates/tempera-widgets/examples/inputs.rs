//! TextInput + NumberField demo.
//!
//! Click the text input to focus, then type. Caret movement (arrows /
//! home / end / word-skip), selection (shift+arrows or mouse drag),
//! and clipboard (Cmd/Ctrl+C/V/X) all come from `bevy_ui_text_input`
//! under tempera's surround. Press Enter to submit.
//!
//! The number field accepts ± stepper clicks and direct typing
//! (filtered to digits + `.` + leading `-`).

use bevy::prelude::*;
use tempera::number_field::ValueChange as NumberFieldChange;
use tempera::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (log_text_contents, log_submits))
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
    text_style: TextInputStyle,
    number_style: NumberFieldStyle,
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
        Text::new("TextInput — click to focus, type, ⌘C/V to copy/paste, Enter to submit"
            .to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));
    let text = spawn_text_input(&mut commands, &text_style, "", "Enter your name…");
    commands.entity(text.surround).insert(ChildOf(root));

    commands.spawn((
        Text::new("NumberField — click ± or type a number".to_string()),
        label_font.clone(),
        TextColor(palette.muted_foreground),
        ChildOf(root),
    ));
    let number = spawn_number_field(
        &mut commands,
        &number_style,
        50.0,
        NumberFieldRange { min: 0.0, max: 100.0 },
        5.0,
    );
    commands
        .entity(number)
        .insert(ChildOf(root))
        .observe(|on: On<NumberFieldChange>| info!("number -> {}", on.value));
}

/// Per-keystroke logger — bevy_ui_text_input uses change detection on
/// `TextInputContents` as its "value changed" signal, so we query
/// that rather than reading an event.
fn log_text_contents(q: Query<&TextInputContents, (With<TextInput>, Changed<TextInputContents>)>) {
    for c in &q {
        info!("text contents -> {:?}", c.get());
    }
}

fn log_submits(mut r: MessageReader<SubmitText>) {
    for ev in r.read() {
        info!("submitted: {:?}", ev.text);
    }
}
