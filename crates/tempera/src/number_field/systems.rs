use bevy::ecs::event::EntityEvent;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::components::{
    NumberField, NumberFieldKind, NumberFieldRange, NumberFieldStep, NumberFieldValue,
};
use crate::text_input::{
    TextInput, TextInputAction, TextInputContents, TextInputEdit, TextInputQueue,
};

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct NumberFieldValueChange {
    #[event_target]
    pub source: Entity,
    pub value: f32,
}

/// On `Activate` of a stepper Button, nudge the parent NumberField's
/// value by ±step, clamped to range.
pub(crate) fn stepper_on_activate(
    on: On<Activate>,
    steppers: Query<(&NumberFieldKind, &ChildOf)>,
    mut fields: Query<
        (
            Entity,
            &mut NumberFieldValue,
            &NumberFieldRange,
            &NumberFieldStep,
        ),
        With<NumberField>,
    >,
    mut commands: Commands,
) {
    let Ok((kind, parent)) = steppers.get(on.entity) else {
        return;
    };
    let Ok((entity, mut value, range, step)) = fields.get_mut(parent.0) else {
        return;
    };
    let delta = match kind {
        NumberFieldKind::Increment => step.0,
        NumberFieldKind::Decrement => -step.0,
    };
    let new_value = (value.0 + delta).clamp(range.min, range.max);
    if (new_value - value.0).abs() > f32::EPSILON {
        value.0 = new_value;
        commands.trigger(NumberFieldValueChange {
            source: entity,
            value: new_value,
        });
    }
}

/// Mirror `NumberFieldValue` into the inner `TextInputNode`'s buffer
/// so the field shows the current number. Only runs when the value
/// component is mutated.
///
/// Layout is two levels: `NumberField → surround (TextInput marker) →
/// inner (TextInputNode + TextInputContents)`.
pub(crate) fn sync_buffer_from_value(
    fields: Query<(&NumberFieldValue, &Children), (With<NumberField>, Changed<NumberFieldValue>)>,
    surrounds: Query<&Children, With<TextInput>>,
    mut inner: Query<(&TextInputContents, &mut TextInputQueue), Without<TextInput>>,
) {
    for (value, kids) in &fields {
        for child in kids.iter() {
            let Ok(surround_kids) = surrounds.get(child) else {
                continue;
            };
            for inner_id in surround_kids.iter() {
                let Ok((contents, mut queue)) = inner.get_mut(inner_id) else {
                    continue;
                };
                let formatted = format!("{}", value.0);
                if contents.get() == formatted {
                    continue;
                }
                // SelectAll on empty buffer panics in cosmic-text —
                // skip when there's nothing to select.
                if !contents.get().is_empty() {
                    queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
                }
                queue.add(TextInputAction::Edit(TextInputEdit::Paste(formatted)));
            }
        }
    }
}

/// When the user edits the text directly, parse and write back to
/// `NumberFieldValue`. Path: `Changed<TextInputContents>` on the inner
/// → walk up to the surround → walk up to the NumberField parent.
pub(crate) fn sync_value_from_buffer(
    inner: Query<(&TextInputContents, &ChildOf), (Without<TextInput>, Changed<TextInputContents>)>,
    surrounds: Query<&ChildOf, With<TextInput>>,
    mut fields: Query<(Entity, &mut NumberFieldValue, &NumberFieldRange), With<NumberField>>,
    mut commands: Commands,
) {
    for (contents, parent) in &inner {
        let Ok(surround_parent) = surrounds.get(parent.0) else {
            continue;
        };
        let Ok((entity, mut value, range)) = fields.get_mut(surround_parent.0) else {
            continue;
        };
        let text = contents.get().trim();
        let Ok(parsed) = text.parse::<f32>() else {
            continue;
        };
        let clamped = parsed.clamp(range.min, range.max);
        if (clamped - value.0).abs() > f32::EPSILON {
            value.0 = clamped;
            commands.trigger(NumberFieldValueChange {
                source: entity,
                value: clamped,
            });
        }
    }
}
