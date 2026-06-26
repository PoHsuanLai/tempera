use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::{
    NumberField, NumberFieldConfig, NumberFieldKind, NumberFieldRange, NumberFieldStep,
    NumberFieldValue,
};
use crate::button::{spawn_button, ButtonContent, ButtonSize, ButtonStyle, ButtonVariant};
use crate::text_input::{spawn_text_input, TextInputFilter, TextInputStyle};
use crate::theme::Spacing;

#[derive(SystemParam)]
pub struct NumberFieldStyle<'w> {
    pub text_input: TextInputStyle<'w>,
    pub button: ButtonStyle<'w>,
    pub spacing: Res<'w, Spacing>,
}

/// Spawn a number field with default styling. Returns the root entity.
/// Observe `super::ValueChange` on it to react to changes.
pub fn spawn_number_field(
    commands: &mut Commands,
    style: &NumberFieldStyle,
    initial: f32,
    range: NumberFieldRange,
    step: f32,
) -> Entity {
    let initial = initial.clamp(range.min, range.max);

    let root = commands
        .spawn((
            NumberField,
            NumberFieldValue(initial),
            range,
            NumberFieldStep(step),
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(style.spacing.xxs),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new("tempera::number_field"),
        ))
        .id();

    // Decrement button.
    let dec = spawn_button(
        commands,
        &style.button,
        ButtonContent::text("−"),
        ButtonVariant::Outline,
    );
    commands
        .entity(dec)
        .insert(ButtonSize::Sm)
        .insert(NumberFieldKind::Decrement)
        .insert(ChildOf(root));

    let handle = spawn_text_input(commands, &style.text_input, format!("{initial}"), "0");
    commands.entity(handle.inner).insert(TextInputFilter::Decimal);
    commands.entity(handle.surround).insert((
        Node {
            width: Val::Px(80.0),
            height: Val::Px(28.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small)),
            padding: UiRect::horizontal(Val::Px(8.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        ChildOf(root),
    ));

    // Increment button.
    let inc = spawn_button(
        commands,
        &style.button,
        ButtonContent::text("+"),
        ButtonVariant::Outline,
    );
    commands
        .entity(inc)
        .insert(ButtonSize::Sm)
        .insert(NumberFieldKind::Increment)
        .insert(ChildOf(root));

    root
}

/// Spawn a number field with caller-specified dimensions and colors.
/// The `[−] [value] [+]` segments share the same background and
/// height, forming a connected strip (Plasticity-style).
///
/// Steppers are plain entities (not `TemperaButton`) so the button
/// repaint system doesn't override the configured background.
pub fn spawn_number_field_configured(
    commands: &mut Commands,
    style: &NumberFieldStyle,
    initial: f32,
    range: NumberFieldRange,
    step: f32,
    config: NumberFieldConfig,
) -> Entity {
    use bevy::picking::events::{Click, Pointer};
    use bevy::ui_widgets::Activate;

    let initial = initial.clamp(range.min, range.max);

    let root = commands
        .spawn((
            NumberField,
            NumberFieldValue(initial),
            range,
            NumberFieldStep(step),
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(config.gap),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new("tempera::number_field"),
        ))
        .id();

    let stepper_text_font = style.text_input.font.text_font(style.text_input.typography.xs);
    let stepper_text_color = TextColor(style.button.palette.muted_foreground);
    let bg = config.bg;
    let bg_hover = config.bg_hover;

    // Decrement — left end-cap.
    let dec = commands
        .spawn((
            NumberFieldKind::Decrement,
            Node {
                width: Val::Px(config.stepper_width),
                height: Val::Px(config.height),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius {
                    top_left: Val::Px(config.border_radius),
                    bottom_left: Val::Px(config.border_radius),
                    top_right: Val::Px(0.0),
                    bottom_right: Val::Px(0.0),
                },
                ..default()
            },
            BackgroundColor(bg),
            Interaction::default(),
            crate::cursor::HoverCursor::default(),
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        Text::new("−"),
        stepper_text_font.clone(),
        stepper_text_color,
        bevy::picking::Pickable::IGNORE,
        ChildOf(dec),
    ));
    commands.entity(dec).observe(
        move |_: On<Pointer<Click>>, mut commands: Commands| {
            commands.trigger(Activate { entity: dec });
        },
    );
    commands.entity(dec).observe(
        move |mut on: On<Pointer<bevy::picking::events::Over>>,
              mut bg_q: Query<&mut BackgroundColor>| {
            if let Ok(mut b) = bg_q.get_mut(dec) {
                b.0 = bg_hover;
            }
            on.propagate(false);
        },
    );
    commands.entity(dec).observe(
        move |mut on: On<Pointer<bevy::picking::events::Out>>,
              mut bg_q: Query<&mut BackgroundColor>| {
            if let Ok(mut b) = bg_q.get_mut(dec) {
                b.0 = bg;
            }
            on.propagate(false);
        },
    );

    // Text input — no individual rounding, shared background, centered text.
    let handle = spawn_text_input(commands, &style.text_input, format!("{initial}"), "0");
    commands.entity(handle.inner).insert((
        TextInputFilter::Decimal,
        bevy_ui_text_input::TextInputNode {
            justification: bevy::text::Justify::Center,
            mode: bevy_ui_text_input::TextInputMode::SingleLine,
            clear_on_submit: false,
            unfocus_on_submit: false,
            ..default()
        },
    ));
    commands.entity(handle.surround).insert((
        Node {
            width: Val::Px(config.input_width),
            height: Val::Px(config.height),
            border: UiRect::ZERO,
            border_radius: BorderRadius::ZERO,
            padding: UiRect::horizontal(Val::Px(4.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(config.input_bg),
        BorderColor::all(Color::NONE),
        ChildOf(root),
    ));

    // Increment — right end-cap.
    let inc = commands
        .spawn((
            NumberFieldKind::Increment,
            Node {
                width: Val::Px(config.stepper_width),
                height: Val::Px(config.height),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius {
                    top_left: Val::Px(0.0),
                    bottom_left: Val::Px(0.0),
                    top_right: Val::Px(config.border_radius),
                    bottom_right: Val::Px(config.border_radius),
                },
                ..default()
            },
            BackgroundColor(bg),
            Interaction::default(),
            crate::cursor::HoverCursor::default(),
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        Text::new("+"),
        stepper_text_font,
        stepper_text_color,
        bevy::picking::Pickable::IGNORE,
        ChildOf(inc),
    ));
    commands.entity(inc).observe(
        move |_: On<Pointer<Click>>, mut commands: Commands| {
            commands.trigger(Activate { entity: inc });
        },
    );
    commands.entity(inc).observe(
        move |mut on: On<Pointer<bevy::picking::events::Over>>,
              mut bg_q: Query<&mut BackgroundColor>| {
            if let Ok(mut b) = bg_q.get_mut(inc) {
                b.0 = bg_hover;
            }
            on.propagate(false);
        },
    );
    commands.entity(inc).observe(
        move |mut on: On<Pointer<bevy::picking::events::Out>>,
              mut bg_q: Query<&mut BackgroundColor>| {
            if let Ok(mut b) = bg_q.get_mut(inc) {
                b.0 = bg;
            }
            on.propagate(false);
        },
    );

    root
}
