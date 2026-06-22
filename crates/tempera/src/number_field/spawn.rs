use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::{
    NumberField, NumberFieldKind, NumberFieldRange, NumberFieldStep, NumberFieldValue,
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

/// Spawn a number field. Returns the root entity. Observe
/// `super::ValueChange` on it to react to changes (stepper clicks or
/// committed text edits).
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

    // The editable text input. The Decimal filter rejects any
    // non-numeric character at the buffer level — anything the user
    // types that's not a digit / `.` / leading `-` is dropped before
    // it ever lands in TextInputContents.
    let handle = spawn_text_input(commands, &style.text_input, format!("{initial}"), "0");
    commands.entity(handle.inner).insert(TextInputFilter::Decimal);
    // Resize to fit between the steppers. Preserve the surround's
    // flex-center so the inner Node sits vertically centered (cosmic-text
    // anchors its content to the top, so the inner is sized to line
    // height and the surround centers it).
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
