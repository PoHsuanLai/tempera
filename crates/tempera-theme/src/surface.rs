//! What a control's fill and edge are made of, independent of which control
//! it is.
//!
//! # Why this is not `ButtonVariant`
//!
//! [`crate::ColorPalette`] answers "what colour is `primary`". A widget still
//! has to decide *whether it has a fill at all*, *whether it has an edge*, and
//! *which palette role it draws from* — and every widget was answering that
//! privately. `ButtonVariant` had seven arms hand-writing six fields each;
//! a text input had one look, hardcoded at spawn, with no way to ask for
//! another.
//!
//! The result was that a consumer wanting a filled borderless input — the
//! search-field shape every app has — spawned tempera's input and then
//! overwrote its `Node`, `BackgroundColor` and `BorderColor` from outside.
//! That does not even work: the repaint system owns the border and paints
//! over it on the next frame.
//!
//! # The three questions
//!
//! [`Surface`] — is there a fill, an edge, both, or neither?
//! [`Emphasis`] — which palette role does it draw from?
//! [`Reactivity`] — does it respond to the pointer with a fill?
//!
//! Two axes would have been tidier, and it was the first thing tried. It does
//! not survive contact with `Ghost` and `Bare`: both are fill-less, edge-less
//! neutral controls, identical in every field but one. `Ghost` lifts to
//! `muted` under the pointer; `Bare` stays transparent and lets an icon tint
//! carry the whole feedback. That is not a different surface and not a
//! different emphasis — it is a separate decision, so it gets a separate
//! name rather than a fourth `Surface` arm that means "like `Bare` but".
//!
//! # What this deliberately does not model
//!
//! `ButtonVariant::Link` does not factor. It is fill-less, edge-less, draws
//! `primary` as *text*, and underlines on hover — a text treatment wearing a
//! button's clothes. Bending the grid to fit one member would cost more than
//! the one arm it saves, so `Link` stays a special case in the button and is
//! named as such here.

use bevy::prelude::*;

use crate::ColorPalette;

/// Whether a control has a fill, an edge, both, or neither.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Surface {
    /// A solid fill, no edge. The shape of a primary action or a filled
    /// input.
    #[default]
    Filled,
    /// No fill, a one-pixel edge. The shape of a secondary action and of
    /// tempera's default text input.
    Outline,
    /// Neither. The control is its content until something happens — see
    /// [`Reactivity`].
    Bare,
}

/// Which palette role a control draws from.
///
/// Names the *role*, not the colour: `Neutral` is the control that carries no
/// weight of its own, and it reads from `muted` / `foreground` rather than
/// from a named accent.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Emphasis {
    /// The one action a surface wants you to take.
    Primary,
    /// A real action, subordinate to the primary one.
    Secondary,
    /// Destructive and irreversible.
    Destructive,
    /// No weight of its own.
    #[default]
    Neutral,
}

/// Whether the pointer gets a fill in response.
///
/// Separate from [`Surface`] because a control can be fill-less at rest and
/// still light up — that is exactly what distinguishes a ghost button from a
/// bare one, and it is a real design choice rather than an implementation
/// detail. A bare control in a dense toolbar that lit up on hover would
/// flicker as the pointer crossed it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Reactivity {
    /// The pointer produces a fill.
    #[default]
    Fills,
    /// The pointer produces nothing here; feedback is the content's job
    /// (an icon tint, an underline).
    Inert,
}

/// A resolved fill/edge/text recipe for one control in one state.
///
/// Every field is a colour ready to write. Nothing downstream re-derives.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceVisuals {
    /// Fill at rest.
    pub fill: Color,
    /// Fill under the pointer.
    pub fill_hover: Color,
    /// Fill while held.
    pub fill_pressed: Color,
    /// Fill when selected but not hovered.
    pub fill_selected: Color,
    /// Edge at rest. [`Color::NONE`] when the surface has no edge.
    pub edge: Color,
    /// Edge width in logical pixels; `0.0` when there is no edge.
    pub edge_width: f32,
    /// Text and icon colour.
    pub text: Color,
}

/// How far a hover moves a colour away from the surface behind it.
///
/// Public because widgets outside the grid step their own colours and must
/// agree on how far a hover is. The checkbox used 0.12, the switch 0.06 and
/// the slider 0.08 — three undocumented numbers for one idea, and no way for
/// a reader to tell whether the differences meant anything. They did not.
///
/// It applies to any *visible resting colour* being moved. It does **not**
/// apply to a control that appears from nothing on hover — a dock divider
/// rests at [`Color::NONE`], so it has no rest state to be different from and
/// answers to legibility against the panels instead.
pub const HOVER: f32 = 0.08;

/// How far a press moves it — further in the same direction, never back
/// toward the surface. See `button::spawn::pressed` for why reversing is
/// wrong.
pub const PRESS: f32 = 0.14;

/// A selected-but-unhovered fill sits below hover, so hover can still lift
/// away from it.
///
/// Deliberately *not* [`HOVER`], and not a candidate for converging with it:
/// the gap between the two is what lets a selected control still respond to
/// the pointer. Making them equal is the defect
/// `a_selected_button_still_responds_to_the_pointer` exists to catch.
pub const SELECT: f32 = 0.04;

/// The three amounts must stay ordered: selection below hover below press.
///
/// A `const` block rather than a `#[test]`, because comparing two constants
/// is a compile-time fact and clippy is right to call a runtime assertion on
/// one pointless. Written as a guard anyway, because the *ordering* is the
/// load-bearing part and it is not obvious from three separate declarations.
///
/// Selection sits below hover so that hovering a selected control still
/// changes it — a selected control that stops responding to the pointer reads
/// as a disabled one. Press sits beyond hover because a press is only ever
/// seen as a change *from* the hovered colour.
const _ORDERED: () = {
    assert!(SELECT < HOVER);
    assert!(HOVER < PRESS);
};

/// The edge width for [`Surface::Outline`].
///
/// A hairline: it answers to the display rather than to the spacing scale,
/// which is why it is a literal here and not a [`crate::Step`].
const EDGE_WIDTH: f32 = 1.0;

impl Emphasis {
    /// The resting fill for a filled control, and the hover fill for a
    /// reactive fill-less one.
    fn fill(self, palette: &ColorPalette) -> Color {
        match self {
            Self::Primary => palette.primary,
            Self::Secondary => palette.secondary,
            Self::Destructive => palette.destructive,
            Self::Neutral => palette.muted,
        }
    }

    /// Text drawn *on* this emphasis's fill.
    fn on_fill(self, palette: &ColorPalette) -> Color {
        match self {
            Self::Primary => palette.primary_foreground,
            Self::Secondary => palette.secondary_foreground,
            Self::Destructive => palette.destructive_foreground,
            Self::Neutral => palette.foreground,
        }
    }
}

/// Resolve a control's colours from the three choices plus the palette.
///
/// `surface_behind` is what the control is drawn on — normally
/// `palette.background`, but a control inside a card or a popover passes that
/// instead, so its states step away from what is actually behind it.
#[must_use]
pub fn visuals(
    surface: Surface,
    emphasis: Emphasis,
    reactivity: Reactivity,
    palette: &ColorPalette,
    surface_behind: Color,
) -> SurfaceVisuals {
    let step = |base: Color, amount: f32| ColorPalette::step(base, surface_behind, amount);
    let role = emphasis.fill(palette);

    match surface {
        Surface::Filled => SurfaceVisuals {
            fill: role,
            fill_hover: step(role, HOVER),
            fill_pressed: step(role, PRESS),
            fill_selected: step(role, SELECT),
            edge: Color::NONE,
            edge_width: 0.0,
            text: emphasis.on_fill(palette),
        },
        Surface::Outline | Surface::Bare => {
            // No resting fill, so the *hover* fill is the first thing drawn.
            // It is the emphasis's own colour rather than a step off it —
            // there is nothing to step from.
            let (hover, pressed, selected) = match reactivity {
                Reactivity::Fills => (role, step(role, HOVER), step(role, SELECT)),
                // An inert control still has to show selection: a selected
                // control indistinguishable from its neighbours is the same
                // defect as one that stops responding to the pointer.
                Reactivity::Inert => (Color::NONE, Color::NONE, step(role, SELECT)),
            };
            SurfaceVisuals {
                fill: Color::NONE,
                fill_hover: hover,
                fill_pressed: pressed,
                fill_selected: selected,
                edge: if surface == Surface::Outline {
                    palette.border
                } else {
                    Color::NONE
                },
                edge_width: if surface == Surface::Outline {
                    EDGE_WIDTH
                } else {
                    0.0
                },
                // Text sits on the page, not on a fill.
                text: match emphasis {
                    Emphasis::Primary => palette.primary,
                    Emphasis::Destructive => palette.destructive,
                    Emphasis::Secondary | Emphasis::Neutral => palette.foreground,
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn both_palettes() -> [ColorPalette; 2] {
        [ColorPalette::dark(), ColorPalette::light()]
    }

    /// Rest, hover and press must be three distinguishable fills, for every
    /// combination the grid can express, in both themes.
    ///
    /// This is the property the whole module owes its callers, and stating it
    /// over the product rather than over a handful of chosen variants is the
    /// point of having a grid at all: `ButtonVariant` could only ever be
    /// tested one arm at a time.
    #[test]
    fn every_reactive_combination_gives_three_distinct_fills() {
        for palette in both_palettes() {
            let bg = palette.background;
            for surface in [Surface::Filled, Surface::Outline, Surface::Bare] {
                for emphasis in [
                    Emphasis::Primary,
                    Emphasis::Secondary,
                    Emphasis::Destructive,
                    Emphasis::Neutral,
                ] {
                    let v = visuals(surface, emphasis, Reactivity::Fills, &palette, bg);
                    assert_ne!(
                        v.fill, v.fill_hover,
                        "{surface:?}/{emphasis:?}: hover is invisible"
                    );
                    assert_ne!(
                        v.fill_hover, v.fill_pressed,
                        "{surface:?}/{emphasis:?}: press is invisible once hovering"
                    );
                }
            }
        }
    }

    /// No state may land on the surface the control is drawn on.
    ///
    /// This is the light-theme defect that motivated `ColorPalette::step`,
    /// restated over the whole grid: a fill equal to the page is a control
    /// that vanished.
    #[test]
    fn no_state_is_ever_the_surface_behind_it() {
        for palette in both_palettes() {
            let bg = palette.background;
            for surface in [Surface::Filled, Surface::Outline, Surface::Bare] {
                for emphasis in [
                    Emphasis::Primary,
                    Emphasis::Secondary,
                    Emphasis::Destructive,
                    Emphasis::Neutral,
                ] {
                    let v = visuals(surface, emphasis, Reactivity::Fills, &palette, bg);
                    for (name, c) in [
                        ("fill", v.fill),
                        ("hover", v.fill_hover),
                        ("pressed", v.fill_pressed),
                        ("selected", v.fill_selected),
                    ] {
                        // A deliberately absent fill is not a vanished one.
                        if c == Color::NONE {
                            continue;
                        }
                        assert_ne!(
                            c.to_srgba().to_vec3(),
                            bg.to_srgba().to_vec3(),
                            "{surface:?}/{emphasis:?}: {name} is the surface colour"
                        );
                    }
                }
            }
        }
    }

    /// An inert control shows nothing under the pointer — that is what it is
    /// for — but a *selected* one is still distinguishable.
    ///
    /// The two halves are one test because holding only the first is how you
    /// get a selected toolbar icon that looks exactly like its neighbours,
    /// and holding only the second defeats the variant.
    #[test]
    fn an_inert_control_ignores_the_pointer_but_not_selection() {
        for palette in both_palettes() {
            let v = visuals(
                Surface::Bare,
                Emphasis::Neutral,
                Reactivity::Inert,
                &palette,
                palette.background,
            );
            assert_eq!(v.fill_hover, Color::NONE, "an inert control filled on hover");
            assert_eq!(v.fill_pressed, Color::NONE);
            assert_ne!(
                v.fill_selected,
                Color::NONE,
                "a selected inert control is invisible"
            );
        }
    }

    /// Only [`Surface::Outline`] draws an edge, and it is the only one with a
    /// non-zero width. A width without a colour, or the reverse, is a
    /// hairline that renders as nothing or a gap that renders as a jump.
    #[test]
    fn an_edge_and_its_width_agree() {
        for palette in both_palettes() {
            for surface in [Surface::Filled, Surface::Outline, Surface::Bare] {
                let v = visuals(
                    surface,
                    Emphasis::Neutral,
                    Reactivity::Fills,
                    &palette,
                    palette.background,
                );
                let has_colour = v.edge != Color::NONE;
                let has_width = v.edge_width > 0.0;
                assert_eq!(
                    has_colour,
                    has_width,
                    "{surface:?}: edge colour and width disagree"
                );
                assert_eq!(has_width, surface == Surface::Outline);
            }
        }
    }

    /// Text on a filled control reads against the fill; text on a fill-less
    /// one reads against the page.
    ///
    /// Getting this backwards is invisible in the dark theme — where
    /// `foreground` and `primary_foreground` are both near-white — and
    /// illegible in the light one, which is why it is asserted over both.
    #[test]
    fn text_answers_to_whatever_is_actually_behind_it() {
        for palette in both_palettes() {
            let filled = visuals(
                Surface::Filled,
                Emphasis::Primary,
                Reactivity::Fills,
                &palette,
                palette.background,
            );
            assert_eq!(filled.text, palette.primary_foreground);

            let bare = visuals(
                Surface::Bare,
                Emphasis::Primary,
                Reactivity::Fills,
                &palette,
                palette.background,
            );
            assert_eq!(bare.text, palette.primary);
        }
    }

    /// The surface argument is honoured, not ignored in favour of
    /// `palette.background`.
    ///
    /// A control inside a popover steps away from the popover. Without this
    /// the argument could be dropped and every existing test would still
    /// pass, because they all pass `background` anyway.
    #[test]
    fn a_control_steps_away_from_the_surface_it_was_given() {
        let palette = ColorPalette::dark();
        let on_page = visuals(
            Surface::Filled,
            Emphasis::Neutral,
            Reactivity::Fills,
            &palette,
            palette.background,
        );
        // A surface *lighter* than the fill flips the direction.
        let on_pale = visuals(
            Surface::Filled,
            Emphasis::Neutral,
            Reactivity::Fills,
            &palette,
            Color::srgb_u8(230, 230, 235),
        );
        assert_ne!(
            on_page.fill_hover, on_pale.fill_hover,
            "the surface argument made no difference"
        );
    }
}
