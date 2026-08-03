use bevy::prelude::*;

use crate::theme::{ColorPalette, HOVER};

/// Tempera marker on a styled text-input root. The behavior layer is
/// [`bevy_ui_text_input::TextInputNode`] (added as a child of the
/// styled surround) — this marker lets the repaint system find the
/// surround entity quickly.
#[derive(Component, Default, Debug)]
pub struct TextInput;

/// Visual variant of a text input.
///
/// A button has had seven of these since the beginning; an input had one,
/// hardcoded at spawn. That asymmetry is what made consumers reach past the
/// widget — dawai's chat wants the filled borderless shape every search
/// field has, so it spawns tempera's input and then overwrites its `Node`,
/// `BackgroundColor` and `BorderColor` from outside.
///
/// **That workaround does not work.** [`super::systems`] owns the border and
/// repaints it whenever focus or the palette changes, so an
/// externally-cleared border comes back on the next such frame. It went
/// unnoticed only because the same patch set the border *width* to zero,
/// which made the wrong colour invisible.
///
/// Three variants rather than the button's seven: an input is a field, not an
/// action, so there is no `Primary`/`Secondary`/`Destructive` hierarchy to
/// express. A red input is a *validation state* — it composes with any of
/// these — and modelling it as a variant would make the two mutually
/// exclusive.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum TextInputVariant {
    /// A page-coloured fill inside a one-pixel edge. Tempera's original and,
    /// until now, only look.
    #[default]
    Outline,
    /// A muted fill and no edge at rest — the search-field and
    /// chat-composer shape.
    Filled,
    /// Neither, until the pointer or focus arrives. For inline editing, where
    /// the field should read as text until someone means to edit it.
    Bare,
}

/// The four colours an input wears in one state.
///
/// Deliberately *not* [`crate::theme::SurfaceVisuals`]. That type answers for
/// a control with rest / hover / press / selected; an input has no press and
/// no selection, and it has **focus**, which no button has. Reusing it would
/// mean four fields carrying nothing and the one field that matters most
/// carrying nothing either.
///
/// The colour *roles* are the theme's — `muted` for a filled surface,
/// `input` for an edge, `ring` for focus, all stepped by
/// [`ColorPalette::step`] — but the *state machine* is the input's own,
/// because its states are.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct InputPaint {
    pub fill: Color,
    pub edge: Color,
}

impl TextInputVariant {
    /// Resting fill.
    ///
    /// `Outline` fills with `background` rather than [`Color::NONE`] so the
    /// field reads as a well against a card or a panel, not only against the
    /// page. That is the one place this diverges from
    /// [`crate::theme::Surface::Outline`], which has no fill at all — a
    /// button sits *on* the page and an input is a hole *in* it.
    fn rest_fill(self, palette: &ColorPalette) -> Color {
        match self {
            Self::Outline => palette.background,
            Self::Filled => palette.muted,
            Self::Bare => Color::NONE,
        }
    }

    /// Resting edge.
    fn rest_edge(self, palette: &ColorPalette) -> Color {
        match self {
            Self::Outline => palette.input,
            Self::Filled | Self::Bare => Color::NONE,
        }
    }

    /// Which of the two channels shows the hover.
    ///
    /// An outlined input has an edge to brighten, so it brightens the edge and
    /// leaves the fill alone. The other two have no edge, so the fill moves
    /// instead. Moving both would double the signal on the one variant that
    /// has both.
    fn hover_is_on_the_edge(self) -> bool {
        matches!(self, Self::Outline)
    }

    /// Resolve every colour for one state.
    ///
    /// Focus outranks hover: an input the user is typing into should look
    /// focused even while the pointer is elsewhere on it.
    pub(crate) fn paint(self, palette: &ColorPalette, hovered: bool, focused: bool) -> InputPaint {
        let fill = self.rest_fill(palette);
        let edge = self.rest_edge(palette);

        if focused {
            // Every variant shows focus the same way, on the edge — including
            // the two with no resting edge. `spawn` reserves the border width
            // for all three precisely so this can happen without reflowing:
            // a `bevy_ui` border is inside the box, so growing one from 0 to
            // 1px would shift the text the user is typing.
            return InputPaint {
                fill,
                edge: palette.ring,
            };
        }
        if hovered {
            return if self.hover_is_on_the_edge() {
                InputPaint {
                    fill,
                    edge: ColorPalette::step(edge, palette.background, HOVER),
                }
            } else {
                InputPaint {
                    // `Bare` has no fill to step from, so it takes the same
                    // `muted` a ghost button takes — the first fill drawn is
                    // the role's own colour, not a step off nothing.
                    fill: match self {
                        Self::Bare => palette.muted,
                        _ => ColorPalette::step(fill, palette.background, HOVER),
                    },
                    edge,
                }
            };
        }
        InputPaint { fill, edge }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [TextInputVariant; 3] = [
        TextInputVariant::Outline,
        TextInputVariant::Filled,
        TextInputVariant::Bare,
    ];

    fn palettes() -> [ColorPalette; 2] {
        [ColorPalette::dark(), ColorPalette::light()]
    }

    /// The variant that motivated the whole component: a filled input has a
    /// fill and no edge, which is what a consumer previously had to patch in
    /// from outside.
    #[test]
    fn a_filled_input_has_a_fill_and_no_edge_at_rest() {
        for p in palettes() {
            let paint = TextInputVariant::Filled.paint(&p, false, false);
            assert_eq!(paint.fill, p.muted);
            assert_eq!(paint.edge, Color::NONE);
        }
    }

    /// The default is what tempera drew before this existed, unchanged.
    ///
    /// This is the pin that makes the change additive: `Outline` is the
    /// `#[default]`, so every existing call site keeps its appearance without
    /// naming a variant.
    #[test]
    fn the_default_variant_still_draws_the_original_input() {
        for p in palettes() {
            let paint = TextInputVariant::default().paint(&p, false, false);
            assert_eq!(paint.fill, p.background);
            assert_eq!(paint.edge, p.input);
        }
    }

    /// Every variant shows focus, and shows it the same way.
    ///
    /// The two edge-less variants are the interesting half: without a
    /// reserved border width they could not do this without reflowing, which
    /// is why `spawn` reserves it for all three.
    #[test]
    fn every_variant_shows_focus_on_its_edge() {
        for p in palettes() {
            for v in ALL {
                let paint = v.paint(&p, false, true);
                assert_eq!(paint.edge, p.ring, "{v:?} did not show focus");
            }
        }
    }

    /// Focus outranks hover: an input being typed into looks focused even
    /// while the pointer rests on it.
    #[test]
    fn focus_wins_over_hover() {
        for p in palettes() {
            for v in ALL {
                assert_eq!(
                    v.paint(&p, true, true),
                    v.paint(&p, false, true),
                    "{v:?} let hover disturb a focused input"
                );
            }
        }
    }

    /// Every variant reacts to the pointer, in exactly one channel, **and the
    /// channel that moves is one you can see**.
    ///
    /// Three halves, and the third was nearly missed. An input that ignores
    /// the pointer reads as read-only, which is why no variant is inert.
    /// Moving *both* channels on `Outline` — the one variant that has both —
    /// would double the signal there relative to the other two.
    ///
    /// And a moved channel has to be *visible*. `ColorPalette::step` adds to
    /// RGB and preserves alpha, so stepping [`Color::NONE`] returns a
    /// different, still fully transparent colour. An earlier version of this
    /// test compared with `!=` alone and passed against two mutations that
    /// painted the hover onto a channel with nothing in it — a widget that
    /// looked reactive in the assertion and was inert on screen. Hence
    /// [`moved_visibly`].
    #[test]
    fn hover_moves_exactly_one_channel_and_the_move_is_visible() {
        for p in palettes() {
            for v in ALL {
                let rest = v.paint(&p, false, false);
                let hot = v.paint(&p, true, false);
                let fill_moved = moved_visibly(rest.fill, hot.fill);
                let edge_moved = moved_visibly(rest.edge, hot.edge);
                assert!(
                    fill_moved ^ edge_moved,
                    "{v:?}: expected exactly one *visible* channel change, \
                     fill_moved={fill_moved} edge_moved={edge_moved}"
                );
            }
        }
    }

    /// Did this channel change in a way a person could see?
    ///
    /// Two fully transparent colours are the same to an eye however far apart
    /// their RGB is, so a change that leaves alpha at zero is not a change.
    fn moved_visibly(before: Color, after: Color) -> bool {
        if before.to_srgba().alpha == 0.0 && after.to_srgba().alpha == 0.0 {
            return false;
        }
        before != after
    }

    /// No state paints a visible channel the same colour as the page.
    ///
    /// The light-theme defect that started this work, restated for the input:
    /// `hover_lift(input, 0.12)` used to land the hovered border on
    /// `(255,255,255)` against a `(255,255,255)` page. A channel deliberately
    /// set to `NONE` is absent, not vanished, so it is skipped.
    #[test]
    fn nothing_an_input_paints_is_ever_the_page_colour() {
        for p in palettes() {
            for v in ALL {
                for (hovered, focused) in [(false, false), (true, false), (false, true)] {
                    let paint = v.paint(&p, hovered, focused);
                    for (name, c) in [("fill", paint.fill), ("edge", paint.edge)] {
                        if c == Color::NONE {
                            continue;
                        }
                        // `Outline` fills with `background` on purpose — it is
                        // a well, not a raised surface — so its fill is
                        // exempt. Its *edge* is what has to stay visible.
                        if name == "fill" && v == TextInputVariant::Outline {
                            continue;
                        }
                        assert_ne!(
                            c.to_srgba().to_vec3(),
                            p.background.to_srgba().to_vec3(),
                            "{v:?} hovered={hovered} focused={focused}: {name} is the page colour"
                        );
                    }
                }
            }
        }
    }
}
