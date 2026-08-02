use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{RadioButton, RadioGroup, ValueChange};

use super::components::ToggleItem;
use crate::theme::ColorPalette;

/// On a `ValueChange<Entity>` for a `RadioGroup`, set `Checked` on the
/// newly-selected button and clear it from every sibling. The headless
/// `RadioGroup` doesn't do this itself — the docs explicitly state the
/// app is responsible. Tempera does it so callers get a working group
/// out of the box.
pub(crate) fn radio_group_self_update(
    on: On<ValueChange<Entity>>,
    groups: Query<&Children, With<RadioGroup>>,
    radios: Query<Entity, With<RadioButton>>,
    mut commands: Commands,
) {
    let Ok(kids) = groups.get(on.source) else {
        return;
    };
    let selected = on.value;
    for child in kids.iter() {
        if radios.get(child).is_ok() {
            if child == selected {
                commands.entity(child).insert(Checked);
            } else {
                commands.entity(child).remove::<Checked>();
            }
        }
    }
}

/// Repaint toggle items + their text labels based on selection +
/// hover + disabled state. Matches shadcn's toggle-group rules:
///
/// - selected → `accent` bg + `accent_foreground` text
/// - hovered (not selected) → `muted` bg + `muted_foreground` text
/// - idle → transparent bg + `foreground` text
///
/// Unfiltered each frame: `Checked` is a marker that's removed on
/// deselect, and `Changed<T>` doesn't fire on removal — using
/// `RemovedComponents<Checked>` plus a filtered query would split this
/// into two paths. Unfiltered is one scalar comparison per item, cheap.
pub(crate) fn repaint_toggle_items(
    palette: Res<ColorPalette>,
    items: Query<
        (
            Entity,
            Has<Checked>,
            &Interaction,
            Has<InteractionDisabled>,
            Option<&Children>,
        ),
        With<ToggleItem>,
    >,
    mut bg: Query<&mut BackgroundColor, With<ToggleItem>>,
    mut text_colors: Query<&mut TextColor, Without<ToggleItem>>,
) {
    for (entity, checked, interaction, disabled, kids) in &items {
        let alpha = if disabled { 0.5 } else { 1.0 };
        let hovered =
            !disabled && matches!(interaction, Interaction::Hovered | Interaction::Pressed);

        let (surface, text) = if checked {
            (palette.accent, palette.accent_foreground)
        } else if hovered {
            (palette.muted, palette.muted_foreground)
        } else {
            (Color::NONE, palette.foreground)
        };
        if let Ok(mut b) = bg.get_mut(entity) {
            *b = BackgroundColor(with_alpha(surface, alpha));
        }
        if let Some(kids) = kids {
            for child in kids.iter() {
                if let Ok(mut tc) = text_colors.get_mut(child) {
                    tc.0 = with_alpha(text, alpha);
                }
            }
        }
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}
