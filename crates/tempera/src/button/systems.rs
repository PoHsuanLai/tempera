//! Single repaint system. Reads each button's variant + size + state
//! (`Pressed` / `Hovered` / `InteractionDisabled`) and writes its
//! `BackgroundColor` + `BorderColor`. The text color sits on the child
//! `Text` node; we only re-tint it when the variant flips, not every
//! frame, to keep this cheap.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};

use super::components::{ButtonVariant, IconTint};
use super::spawn::{variant_visuals, ButtonStyle, VariantVisuals};

/// Read-side projection over a button entity. Naming the projection
/// keeps the iteration loop concise and surfaces the dependency set.
#[derive(QueryData)]
pub struct ButtonPaint {
    entity: Entity,
    variant: Option<&'static ButtonVariant>,
    interaction: &'static Interaction,
    pressed: Has<Pressed>,
    disabled: Has<InteractionDisabled>,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct ButtonPaintMut {
    bg: &'static mut BackgroundColor,
    border: &'static mut BorderColor,
}

/// Repaint every `Button` based on its current state. Cheap — only
/// writes when the resulting color differs from the current one.
pub fn repaint_buttons(
    style: ButtonStyle,
    mut buttons: Query<(ButtonPaint, ButtonPaintMut), With<super::TemperaButton>>,
) {
    for (state, mut visual) in &mut buttons {
        let variant = state.variant.copied().unwrap_or_default();
        let v = variant_visuals(variant, &style.palette);

        let (bg, border) = resting_or_state(&v, state.interaction, state.pressed, state.disabled);

        if visual.bg.0 != bg {
            *visual.bg = BackgroundColor(bg);
        }
        // BorderColor compares structurally; comparing the all-side color
        // is enough for our uniform borders.
        if visual.border.top != border {
            *visual.border = BorderColor::all(border);
        }
    }
}

fn resting_or_state(
    v: &VariantVisuals,
    interaction: &Interaction,
    pressed: bool,
    disabled: bool,
) -> (Color, Color) {
    if disabled {
        return (
            with_alpha(v.bg_resting, 0.5),
            with_alpha(v.border_resting, 0.5),
        );
    }
    if pressed {
        return (v.bg_pressed, v.border_resting);
    }
    match interaction {
        Interaction::Hovered => (v.bg_hover, v.border_resting),
        Interaction::Pressed => (v.bg_pressed, v.border_resting),
        Interaction::None => (v.bg_resting, v.border_resting),
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}

/// For buttons with [`IconTint`], write the appropriate icon color
/// into the `ImageNode.color` of any direct image child. Pairs with
/// the [`ButtonVariant::Ghost`] no-surface look — dawai's toolbar
/// glyphs (sidebar toggles, chevrons) recolor on hover instead of
/// filling a background.
pub fn repaint_icon_tints(
    buttons: Query<
        (&IconTint, &Interaction, Has<InteractionDisabled>, &Children),
        With<super::TemperaButton>,
    >,
    mut icons: Query<&mut ImageNode>,
) {
    for (tint, interaction, disabled, kids) in &buttons {
        let mut color = match interaction {
            Interaction::Hovered | Interaction::Pressed if !disabled => tint.hover,
            _ => tint.resting,
        };
        if disabled {
            color = with_alpha(color, 0.5);
        }
        for child in kids.iter() {
            if let Ok(mut image) = icons.get_mut(child) {
                if image.color != color {
                    image.color = color;
                }
            }
        }
    }
}
