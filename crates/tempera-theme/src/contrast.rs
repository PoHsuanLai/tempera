//! Moving a colour away from the surface behind it.
//!
//! # The rule
//!
//! A widget that has to *look different* under the pointer needs a colour a
//! step away from where it rests. "A step" is not "brighter": on a light
//! theme, brighter means **toward the page**, and a control that brightens on
//! hover against a white page fades out instead of standing up.
//!
//! [`ColorPalette::step`] takes the surface the colour sits on and moves away
//! from it. On a dark theme that is lighter; on a light theme, darker. One
//! call site, two themes, one intent.
//!
//! # Why this replaced `hover_lift`
//!
//! `hover_lift` added a fixed amount to each channel and clamped. That is
//! correct on a dark palette and wrong on a light one, and the failure was
//! not subtle:
//!
//! | call | dark | light |
//! |---|---|---|
//! | input border on hover, `input` +0.12 | `(82,82,91)` → `(113,113,122)` | `(228,228,231)` → **`(255,255,255)`** |
//! | selected button, `muted` +0.04 | `(64,64,67)` → `(74,74,77)` | `(244,244,245)` → **`(254,254,255)`** |
//!
//! In light mode the hovered input border became *the page colour* — the
//! border vanished on hover rather than strengthening — and a selected button
//! landed one point off white on a white background. Both are invisible
//! rather than merely off.
//!
//! The same flaw hit saturated colours in *both* themes. `lighten` on
//! `destructive` `(245,90,90)` clamped red at 255 while green and blue kept
//! moving, so the hover state was a different hue, not a different value.
//!
//! # Direction is chosen once, not per call
//!
//! Away-from-the-surface is the intent, but a colour can be too close to the
//! far end to move: `primary` in the dark theme is `(250,250,250)`, and
//! lighter has five points of room. Such a colour reverses — it separates by
//! going *darker*, which reads as a change just as well.
//!
//! That fallback is decided from the room available at [`MAX_STEP`], **not at
//! the requested amount**, and that detail is the whole reason the ramp is
//! usable. Deciding per call makes the direction flip mid-ramp: light
//! `primary` `(24,24,27)` has room to go darker at 0.04 and 0.08 but not at
//! 0.12, so a per-call rule returns `14 → 4 → 55` and a pressed button lands
//! *lighter* than a hovered one. Choosing once gives `34 → 44 → 55`.
//! `a_ramp_never_doubles_back` is that property.
//!
//! # What this is not
//!
//! Not a contrast-ratio solver. It does not guarantee a WCAG ratio against
//! the surface, and a caller that needs legible *text* wants a foreground
//! token from the palette, not a step off the background. This answers one
//! question — "same colour, visibly moved" — which is what interaction
//! feedback is.

use bevy::prelude::*;

use crate::ColorPalette;

/// The largest step any caller asks for, and the headroom the direction
/// choice is made against.
///
/// It is a constant rather than a parameter because direction has to be a
/// property of the `(base, surface)` pair alone. If it varied with the
/// requested amount, the same pair would move opposite ways at different
/// points in one ramp — see the module docs.
///
/// Raising it makes the reverse-direction fallback trigger for more colours.
/// A caller passing more than this still gets a sensible answer, just one
/// whose direction was chosen against a smaller step than it took.
pub const MAX_STEP: f32 = 0.12;

/// Relative luminance, Rec. 709 coefficients over linearised sRGB.
///
/// Used only to compare two colours' lightness, so the absolute value never
/// escapes this module.
fn luminance(c: Color) -> f32 {
    let s = c.to_srgba();
    let lin = |u: f32| {
        if u <= 0.040_45 {
            u / 12.92
        } else {
            ((u + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(s.red) + 0.7152 * lin(s.green) + 0.0722 * lin(s.blue)
}

/// `true` if a step off `base` should go lighter, `false` darker.
fn goes_lighter(base: Color, surface: Color) -> bool {
    let b = base.to_srgba();
    let channels = [b.red, b.green, b.blue];
    // Away from the surface is the intent.
    let mut lighter = luminance(base) >= luminance(surface);

    // ...unless there is no room that way and there is room the other way.
    // Measured at MAX_STEP so one pair always answers the same, whatever
    // amount this particular call asked for.
    let room_up = 1.0 - channels.iter().copied().fold(f32::MIN, f32::max);
    let room_down = channels.iter().copied().fold(f32::MAX, f32::min);
    if lighter && room_up < MAX_STEP && room_down >= MAX_STEP {
        lighter = false;
    } else if !lighter && room_down < MAX_STEP && room_up >= MAX_STEP {
        lighter = true;
    }
    lighter
}

impl ColorPalette {
    /// Move `base` away from the `surface` behind it by `amount`.
    ///
    /// `amount` is a fraction of the full channel range — 0.04 is a faint
    /// nudge, 0.12 a clear one. Alpha is preserved.
    ///
    /// ```
    /// # use tempera_theme::ColorPalette;
    /// # use bevy::prelude::*;
    /// let dark = ColorPalette::dark();
    /// // On a dark page a muted chip separates by getting lighter...
    /// let hovered = ColorPalette::step(dark.muted, dark.background, 0.08);
    /// assert!(hovered.to_srgba().red > dark.muted.to_srgba().red);
    ///
    /// // ...and on a light page, by getting darker. Same call.
    /// let light = ColorPalette::light();
    /// let hovered = ColorPalette::step(light.muted, light.background, 0.08);
    /// assert!(hovered.to_srgba().red < light.muted.to_srgba().red);
    /// ```
    #[must_use]
    pub fn step(base: Color, surface: Color, amount: f32) -> Color {
        let sign = if goes_lighter(base, surface) { 1.0 } else { -1.0 };
        let d = sign * amount;
        let s = base.to_srgba();
        Color::srgba(
            (s.red + d).clamp(0.0, 1.0),
            (s.green + d).clamp(0.0, 1.0),
            (s.blue + d).clamp(0.0, 1.0),
            s.alpha,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(c: Color) -> f32 {
        c.to_srgba().red
    }

    /// The property the whole module exists for, stated over both shipped
    /// palettes rather than one hand-picked pair.
    #[test]
    fn a_step_always_moves_away_from_the_surface() {
        for palette in [ColorPalette::dark(), ColorPalette::light()] {
            let bg = palette.background;
            for base in [palette.muted, palette.input, palette.secondary] {
                let moved = ColorPalette::step(base, bg, 0.08);
                let before = (luminance(base) - luminance(bg)).abs();
                let after = (luminance(moved) - luminance(bg)).abs();
                assert!(
                    after > before,
                    "step moved {base:?} toward the surface {bg:?}: \
                     separation {before} -> {after}"
                );
            }
        }
    }

    /// The luminance comparison decides direction on its own, with the
    /// headroom fallback out of the picture.
    ///
    /// This test exists because the obvious ones do not pin it. Every colour
    /// in the light palette sits within [`MAX_STEP`] of white — `muted` has
    /// 0.039 of headroom, `input` 0.094 — so the fallback flips them darker
    /// regardless of what the luminance comparison said. Replacing that
    /// comparison with `true` keeps all six other tests in this module green.
    ///
    /// A mid-grey has half the range free in both directions, so nothing
    /// rescues a wrong answer: on a lighter surface it must darken, and on a
    /// darker surface lighten. That is a consumer's case, not tempera's own —
    /// a track header on a light panel, a chip on a card — which is exactly
    /// why the shipped palettes cannot stand in for it.
    #[test]
    fn direction_follows_the_surface_when_there_is_room_either_way() {
        let grey = Color::srgb_u8(128, 128, 128);
        let light_card = Color::srgb_u8(244, 244, 245);
        let dark_card = Color::srgb_u8(24, 24, 27);

        assert!(
            luminance(ColorPalette::step(grey, light_card, 0.08)) < luminance(grey),
            "a mid-grey on a lighter surface must darken"
        );
        assert!(
            luminance(ColorPalette::step(grey, dark_card, 0.08)) > luminance(grey),
            "a mid-grey on a darker surface must lighten"
        );
    }

    /// The exact defect that motivated this: in the light theme, hovering an
    /// input used to paint its border the same colour as the page.
    ///
    /// `hover_lift(input, 0.12)` gave `(255,255,255)` on a `(255,255,255)`
    /// background. Pinned as a regression, because it is the kind of thing a
    /// later "simplify the colour helpers" pass would reintroduce.
    #[test]
    fn a_hovered_input_border_does_not_dissolve_into_a_light_page() {
        let p = ColorPalette::light();
        let hovered = ColorPalette::step(p.input, p.background, 0.12);
        assert_ne!(
            hovered.to_srgba().to_vec3(),
            p.background.to_srgba().to_vec3(),
            "hovered input border became the page colour"
        );
        assert!(
            luminance(hovered) < luminance(p.input),
            "on a light page a hovered border must darken"
        );
    }

    /// Direction is a property of the (base, surface) pair, so a sequence of
    /// increasing amounts is monotonic.
    ///
    /// Without this, light `primary` runs 34 → 44 → 55 one way and
    /// 14 → 4 → 55 the other: a pressed button lighter than a hovered one.
    /// See [`MAX_STEP`].
    #[test]
    fn a_ramp_never_doubles_back() {
        for palette in [ColorPalette::dark(), ColorPalette::light()] {
            let bg = palette.background;
            for base in [
                palette.muted,
                palette.primary,
                palette.input,
                palette.destructive,
            ] {
                let ramp: Vec<f32> = [0.04, 0.08, 0.12]
                    .iter()
                    .map(|&a| r(ColorPalette::step(base, bg, a)))
                    .collect();
                let up = ramp.windows(2).all(|w| w[1] >= w[0]);
                let down = ramp.windows(2).all(|w| w[1] <= w[0]);
                assert!(up || down, "non-monotonic ramp for {base:?}: {ramp:?}");
            }
        }
    }

    /// A colour with no headroom the natural way still moves.
    ///
    /// `primary` in the dark theme is `(250,250,250)` — five points off the
    /// ceiling. `lighten` gave it +5 of a requested +20 and the hover state
    /// was very nearly invisible; the reverse fallback gives a full step.
    #[test]
    fn a_near_white_colour_separates_by_darkening() {
        let p = ColorPalette::dark();
        let moved = ColorPalette::step(p.primary, p.background, 0.08);
        assert!(
            luminance(moved) < luminance(p.primary),
            "a near-white base should reverse rather than clamp"
        );
        let delta = (r(p.primary) - r(moved)).abs();
        assert!(
            (delta - 0.08).abs() < 1e-4,
            "expected a full 0.08 step, moved {delta}"
        );
    }

    /// Every channel moves by the same amount, so the hue survives.
    ///
    /// `lighten(destructive, 0.08)` clamped red at 255 while green and blue
    /// moved, turning a red button pink on hover. The reverse-direction
    /// fallback is what makes this possible: a saturated colour is only ever
    /// stepped in the direction that has room for all three channels.
    #[test]
    fn stepping_a_saturated_colour_keeps_its_hue() {
        for palette in [ColorPalette::dark(), ColorPalette::light()] {
            let base = palette.destructive;
            let moved = ColorPalette::step(base, palette.background, 0.08);
            let (b, m) = (base.to_srgba(), moved.to_srgba());
            let dr = m.red - b.red;
            let dg = m.green - b.green;
            let db = m.blue - b.blue;
            assert!(
                (dr - dg).abs() < 1e-4 && (dg - db).abs() < 1e-4,
                "channels drifted apart: {dr}, {dg}, {db} — hue shifted"
            );
        }
    }

    /// Alpha is a separate decision from value; stepping must not touch it.
    ///
    /// `border` in the dark theme is `srgba(1,1,1,0.08)`, so a helper that
    /// dropped alpha would turn a hairline into a solid white rule.
    #[test]
    fn stepping_preserves_alpha() {
        let p = ColorPalette::dark();
        let moved = ColorPalette::step(p.border, p.background, 0.08);
        assert!((moved.to_srgba().alpha - p.border.to_srgba().alpha).abs() < 1e-6);
    }

    /// A step of nothing is the colour itself — including for a base whose
    /// direction had to be reversed.
    #[test]
    fn a_zero_step_is_the_identity() {
        let p = ColorPalette::dark();
        for base in [p.muted, p.primary, p.destructive] {
            let moved = ColorPalette::step(base, p.background, 0.0);
            assert_eq!(moved.to_srgba().to_vec3(), base.to_srgba().to_vec3());
        }
    }
}
