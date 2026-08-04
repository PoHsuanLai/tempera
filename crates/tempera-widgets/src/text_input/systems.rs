//! Tempera paints the surround (border + bg); editing behavior lives in
//! `bevy_ui_text_input`. The repaint system tints the border on focus
//! and hover.

use bevy::ecs::query::QueryData;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use super::components::{TextInput, TextInputVariant};
use crate::theme::ColorPalette;

/// Paint every input's fill and border.
///
/// One unfiltered pass, where there used to be a `dirty` query plus an
/// `all` query plus a `handled` vec to keep them from double-painting.
/// That structure existed because focus change is not a per-entity fact
/// and could not be a query filter — the same reason a palette swap could
/// not be. Both now live in the run condition, so there is one loop, and
/// each write compares before it lands.
///
/// The fill is painted here rather than left where `spawn` put it because a
/// variant that carries hover on the fill — [`TextInputVariant::Filled`] —
/// needs it to move. That is also what makes the old consumer-side re-skin
/// unnecessary *and* impossible: this system is the single writer of both
/// channels.
/// One input's state and the two channels painted from it.
///
/// A single query rather than a state query plus a paint query: both would
/// be filtered `With<TextInput>` and one is `&mut`, so they would overlap and
/// trip B0001. A `Changed` filter does not make two queries disjoint — only a
/// component filter does. Bundling also keeps the parameter list under
/// clippy's complexity bar, the same way `ButtonPaint` does for the button.
#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct InputPaintQuery {
    entity: Entity,
    interaction: &'static Interaction,
    variant: Option<&'static TextInputVariant>,
    disabled: Has<InteractionDisabled>,
    bg: &'static mut BackgroundColor,
    border: &'static mut BorderColor,
}

pub(crate) fn repaint_text_input(
    palette: Res<ColorPalette>,
    focus: Res<InputFocus>,
    mut inputs: Query<InputPaintQuery, With<TextInput>>,
) {
    let focused = focus.get();
    for mut input in &mut inputs {
        let (entity, interaction, variant, disabled) = (
            input.entity,
            input.interaction,
            input.variant,
            input.disabled,
        );
        let (bg, border) = (&mut input.bg, &mut input.border);
        let alpha = if disabled { 0.5 } else { 1.0 };
        let hovered =
            !disabled && matches!(interaction, Interaction::Hovered | Interaction::Pressed);

        let paint =
            variant
                .copied()
                .unwrap_or_default()
                .paint(&palette, hovered, focused == Some(entity));

        let want_bg = BackgroundColor(with_alpha(paint.fill, alpha));
        if **bg != want_bg {
            **bg = want_bg;
        }
        let want_border = BorderColor::all(with_alpha(paint.edge, alpha));
        if **border != want_border {
            **border = want_border;
        }
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_input::TextInputVariant;
    use crate::theme::ThemePlugin;

    /// Build an app running the real repaint system under its real run
    /// condition, so the gating is exercised too.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(ThemePlugin)
            .init_resource::<InputFocus>()
            .add_systems(
                Update,
                repaint_text_input.run_if(
                    crate::theme::repaint_needed::<TextInput>
                        .or_else(resource_changed::<InputFocus>),
                ),
            );
        app
    }

    fn spawn(app: &mut App, variant: TextInputVariant) -> Entity {
        app.world_mut()
            .spawn((
                TextInput,
                variant,
                Node::default(),
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                Interaction::None,
            ))
            .id()
    }

    /// Compare colours by value, not by which `Color` variant holds them.
    ///
    /// `bevy`'s `Color` is an enum over colour spaces and its `PartialEq`
    /// compares the variant, so `Color::NONE` — a `LinearRgba` — is unequal
    /// to the visually identical `Srgba` the paint path produces. Only tests
    /// notice: production compares two values that both came from
    /// `with_alpha`, so they are always the same variant.
    fn colours(app: &App, e: Entity) -> (Srgba, Srgba) {
        (
            app.world().get::<BackgroundColor>(e).unwrap().0.to_srgba(),
            app.world().get::<BorderColor>(e).unwrap().top.to_srgba(),
        )
    }

    /// The system writes **both** channels, not just the border.
    ///
    /// It painted only `BorderColor` before this change, which is why a
    /// filled variant was not expressible: the fill it needs was whatever
    /// `spawn` happened to leave behind, and nothing could move it on hover.
    #[test]
    fn the_repaint_owns_the_fill_as_well_as_the_edge() {
        let mut app = app();
        let e = spawn(&mut app, TextInputVariant::Filled);
        app.update();

        let palette = app.world().resource::<crate::theme::ColorPalette>().clone();
        let (fill, edge) = colours(&app, e);
        assert_eq!(
            fill,
            palette.muted.to_srgba(),
            "the system did not paint the fill"
        );
        assert_eq!(edge, Color::NONE.to_srgba());
    }

    /// A consumer cannot re-skin an input from outside — and no longer needs
    /// to.
    ///
    /// This is the defect that started this work, pinned as a regression.
    /// dawai's chat panel spawned an input and immediately overwrote its
    /// `BackgroundColor` and `BorderColor`; the repaint puts them back. The
    /// test asserts the overwrite loses, because the fix is to ask for
    /// [`TextInputVariant::Filled`] rather than to make the system yield.
    #[test]
    fn an_outside_write_does_not_survive_the_repaint() {
        let mut app = app();
        let e = spawn(&mut app, TextInputVariant::default());
        app.update();
        let (fill_before, edge_before) = colours(&app, e);

        // What the old consumer-side patch did.
        app.world_mut()
            .entity_mut(e)
            .insert(BackgroundColor(Color::srgb(1.0, 0.0, 1.0)))
            .insert(BorderColor::all(Color::NONE));
        // Any repaint trigger: a palette swap stands in for focus moving.
        let palette = app.world().resource::<crate::theme::ColorPalette>().clone();
        app.insert_resource(palette);
        app.update();

        assert_eq!(
            colours(&app, e),
            (fill_before, edge_before),
            "an outside write survived — the widget has two writers again"
        );
    }

    /// Focus reaches the entity through the resource, for a variant with no
    /// resting edge.
    ///
    /// The pure-logic tests prove `paint` returns `ring`; this proves the
    /// system delivers it, including that the run condition fires on a focus
    /// change rather than only on interaction.
    #[test]
    fn focusing_a_filled_input_draws_its_ring() {
        let mut app = app();
        let e = spawn(&mut app, TextInputVariant::Filled);
        app.update();
        assert_eq!(colours(&app, e).1, Color::NONE.to_srgba());

        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(e, bevy::input_focus::FocusCause::Pressed);
        app.update();

        let palette = app.world().resource::<crate::theme::ColorPalette>().clone();
        assert_eq!(colours(&app, e).1, palette.ring.to_srgba());
    }
}
