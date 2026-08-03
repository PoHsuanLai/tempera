//! Single repaint system. Reads each button's variant + size + state
//! (`Pressed` / `Hovered` / `InteractionDisabled`) and writes its
//! `BackgroundColor` + `BorderColor`. The text color sits on the child
//! `Text` node; we only re-tint it when the variant flips, not every
//! frame, to keep this cheap.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};

use super::components::{ButtonVariant, IconTint, Selected};
use super::spawn::{ButtonStyle, VariantVisuals, variant_visuals};

/// Read-side projection over a button entity. Naming the projection
/// keeps the iteration loop concise and surfaces the dependency set.
#[derive(QueryData)]
pub struct ButtonPaint {
    entity: Entity,
    variant: Option<&'static ButtonVariant>,
    interaction: &'static Interaction,
    pressed: Has<Pressed>,
    disabled: Has<InteractionDisabled>,
    selected: Has<Selected>,
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

        let (bg, border) = resting_or_state(
            &v,
            &style.palette,
            state.interaction,
            state.pressed,
            state.disabled,
            state.selected,
        );

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

/// Resolve the fill and border for one button's current state.
///
/// The order is deliberate: disabled beats everything, then pressed, then
/// hover, then selection, then rest. Selection sits *below* hover so a
/// selected button still lights up under the pointer — a selected control
/// that stops responding reads as disabled.
fn resting_or_state(
    v: &VariantVisuals,
    palette: &crate::theme::ColorPalette,
    interaction: &Interaction,
    pressed: bool,
    disabled: bool,
    selected: bool,
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
        Interaction::None if selected => (v.bg_selected(palette), v.border_resting),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::button::{Button, ButtonSize, ButtonVariant, TemperaButton};
    use crate::theme::{ColorPalette, ThemePlugin};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(ThemePlugin)
            .add_systems(Update, repaint_buttons);
        app
    }

    fn spawn(app: &mut App, variant: ButtonVariant, selected: bool) -> Entity {
        let e = app
            .world_mut()
            .spawn((
                Button,
                TemperaButton,
                variant,
                ButtonSize::Icon,
                Node::default(),
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                Interaction::None,
            ))
            .id();
        if selected {
            app.world_mut().entity_mut(e).insert(Selected);
        }
        e
    }

    fn bg(app: &App, e: Entity) -> Color {
        app.world().get::<BackgroundColor>(e).unwrap().0
    }

    #[test]
    fn selecting_a_button_changes_its_fill_without_anyone_writing_a_colour() {
        // The whole point of the marker. dawai's icon buttons carried an
        // `IconHover { idle, hover }` of *resolved colours*, so every surface
        // with a selection had to recompute both and write them back as state
        // changed — which also gave `BackgroundColor` two writers.
        let mut app = app();
        let idle = spawn(&mut app, ButtonVariant::Ghost, false);
        let picked = spawn(&mut app, ButtonVariant::Ghost, true);
        app.update();

        assert_ne!(bg(&app, idle), bg(&app, picked));
        assert_eq!(
            bg(&app, idle),
            Color::NONE,
            "a ghost button rests invisible"
        );
    }

    #[test]
    fn selection_survives_a_palette_swap() {
        // The failure this replaces: dawai's toolbar wrote `srgba(1,1,1,0.12)`
        // for its active tab, so a recoloured theme could never reach it.
        let mut app = app();
        let picked = spawn(&mut app, ButtonVariant::Ghost, true);
        app.update();
        let before = bg(&app, picked);

        app.world_mut().resource_mut::<ColorPalette>().muted = Color::srgb(0.9, 0.1, 0.4);
        app.update();

        assert_ne!(
            bg(&app, picked),
            before,
            "the selected fill follows the theme"
        );
    }

    /// Rest, hover and press must be three distinguishable fills — in both
    /// shipped themes.
    ///
    /// This is the property the `hover`/`pressed` pair exists to hold, and
    /// neither of the two obvious implementations holds it everywhere. The
    /// old one (`lighten` for hover, `darken` for press) always moved hover
    /// toward white, so on the light theme a hovered button drifted into the
    /// page. Reversing press instead of extending it fails the other way: a
    /// press step *toward* the surface has nowhere to go for a fill already
    /// near it, and `secondary` on the light theme lands exactly on the page
    /// colour.
    ///
    /// `Secondary` is the variant under test because it is the one with a
    /// resting fill close to the light theme's background — the case both
    /// wrong answers break on. `Ghost` would pass either way.
    #[test]
    fn rest_hover_and_press_are_three_different_fills_in_both_themes() {
        for palette in [ColorPalette::dark(), ColorPalette::light()] {
            let mut app = app();
            app.insert_resource(palette.clone());
            let e = spawn(&mut app, ButtonVariant::Secondary, false);

            app.update();
            let resting = bg(&app, e);

            *app.world_mut().get_mut::<Interaction>(e).unwrap() = Interaction::Hovered;
            app.update();
            let hovered = bg(&app, e);

            *app.world_mut().get_mut::<Interaction>(e).unwrap() = Interaction::Pressed;
            app.update();
            let held = bg(&app, e);

            assert_ne!(resting, hovered, "hover is invisible");
            assert_ne!(hovered, held, "press is invisible once already hovering");
            for (name, c) in [("resting", resting), ("hovered", hovered), ("held", held)] {
                assert_ne!(
                    c.to_srgba().to_vec3(),
                    palette.background.to_srgba().to_vec3(),
                    "the {name} fill is the page colour — the button vanishes"
                );
            }
        }
    }

    #[test]
    fn a_selected_button_still_responds_to_the_pointer() {
        // Selection sits *below* hover in `resting_or_state`. A selected
        // control that stops reacting to the pointer reads as disabled.
        let mut app = app();
        let e = spawn(&mut app, ButtonVariant::Ghost, true);
        app.update();
        let resting = bg(&app, e);

        *app.world_mut().get_mut::<Interaction>(e).unwrap() = Interaction::Hovered;
        app.update();

        assert_ne!(bg(&app, e), resting);
    }

    #[test]
    fn deselecting_returns_the_button_to_rest() {
        // `Changed<T>` does not fire on removal, so a naive implementation
        // leaves the button stuck looking selected.
        let mut app = app();
        let e = spawn(&mut app, ButtonVariant::Ghost, true);
        app.update();
        let chosen = bg(&app, e);

        app.world_mut().entity_mut(e).remove::<Selected>();
        app.update();

        assert_ne!(bg(&app, e), chosen);
        assert_eq!(bg(&app, e), Color::NONE);
    }

    #[test]
    fn a_surfaceless_variant_still_shows_its_selection() {
        // `Bare` and `Link` have no hover fill to hold, so `bg_selected`
        // falls back to `muted`. Without that a selected Bare button is
        // indistinguishable from its neighbours — which is what the variant
        // is *for* until it gets selected.
        let mut app = app();
        for variant in [ButtonVariant::Bare, ButtonVariant::Link] {
            let e = spawn(&mut app, variant, true);
            app.update();
            assert_ne!(bg(&app, e), Color::NONE, "{variant:?} must show selection");
        }
    }

    #[test]
    fn disabled_beats_selected() {
        let mut app = app();
        let e = spawn(&mut app, ButtonVariant::Default, true);
        app.world_mut()
            .entity_mut(e)
            .insert(bevy::ui::InteractionDisabled);
        app.update();

        let plain_selected = spawn(&mut app, ButtonVariant::Default, true);
        app.update();
        assert_ne!(bg(&app, e), bg(&app, plain_selected));
    }
}
