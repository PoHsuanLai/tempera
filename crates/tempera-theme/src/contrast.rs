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
        let sign = if goes_lighter(base, surface) {
            1.0
        } else {
            -1.0
        };
        let d = sign * amount;
        let s = base.to_srgba();
        Color::srgba(
            (s.red + d).clamp(0.0, 1.0),
            (s.green + d).clamp(0.0, 1.0),
            (s.blue + d).clamp(0.0, 1.0),
            s.alpha,
        )
    }

    /// Move `base` **toward** the `surface` behind it by `amount`: the
    /// disabled direction.
    ///
    /// [`step`](Self::step) separates a colour from its background so it reads
    /// as a change. This does the opposite — it lets a colour recede, which is
    /// what "unavailable" looks like: still legible enough to read, plainly not
    /// a thing you can use.
    ///
    /// `amount` is the fraction of the distance to travel: `0.0` is `base`
    /// unchanged, `1.0` is `surface` exactly (and therefore invisible). `0.5`
    /// is the useful middle. Alpha is preserved from `base`.
    ///
    /// # Why this is not `step` with a negative amount
    ///
    /// Two reasons, and both would be bugs.
    ///
    /// `step` picks its direction from luminance and then **reverses** when the
    /// colour has no headroom that way — that fallback is right for separation
    /// and wrong here, because it would push a disabled colour *away* from the
    /// surface and make it stand out more than the enabled one. Toward the
    /// surface always has room, because the surface is the destination.
    ///
    /// `step` also adds a constant to each channel, which changes hue on
    /// saturated colours (the `lighten`-clamping flaw the module docs record).
    /// Interpolating cannot overshoot and cannot skew hue, because every
    /// channel arrives at its destination together.
    ///
    /// ```
    /// # use tempera_theme::ColorPalette;
    /// # use bevy::prelude::*;
    /// // Halfway to the page, in either theme.
    /// for palette in [ColorPalette::dark(), ColorPalette::light()] {
    ///     let dim = ColorPalette::toward(palette.foreground, palette.background, 0.5);
    ///     let (fg, bg) = (palette.foreground.to_srgba(), palette.background.to_srgba());
    ///     // Strictly between the two, never past either end.
    ///     assert!((dim.to_srgba().red - fg.red).abs() < (bg.red - fg.red).abs());
    /// }
    /// ```
    #[must_use]
    pub fn toward(base: Color, surface: Color, amount: f32) -> Color {
        let t = amount.clamp(0.0, 1.0);
        let (b, s) = (base.to_srgba(), surface.to_srgba());
        Color::srgba(
            b.red + (s.red - b.red) * t,
            b.green + (s.green - b.green) * t,
            b.blue + (s.blue - b.blue) * t,
            // `base`'s alpha, not the surface's: a disabled row recedes in
            // colour, it does not become partly transparent. Fading alpha
            // instead would let whatever sits behind the surface bleed through,
            // which is a different effect and one that stacks badly on overlaps.
            b.alpha,
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

    /// The mirror of `a_step_always_moves_away_from_the_surface`, and the
    /// property a disabled row rests on.
    #[test]
    fn toward_always_moves_closer_to_the_surface() {
        for palette in [ColorPalette::dark(), ColorPalette::light()] {
            let bg = palette.background;
            for base in [
                palette.foreground,
                palette.muted_foreground,
                palette.primary,
            ] {
                let moved = ColorPalette::toward(base, bg, 0.5);
                let before = (luminance(base) - luminance(bg)).abs();
                let after = (luminance(moved) - luminance(bg)).abs();
                assert!(
                    after < before,
                    "toward moved {base:?} away from the surface {bg:?}: \
                     separation {before} -> {after}"
                );
            }
        }
    }

    /// Why this is not `step` with a negative amount.
    ///
    /// `step` reverses direction when a colour has no headroom the way it
    /// wanted to go — correct for separation, fatal here. `foreground` in the
    /// dark palette is (250,250,250), five points from white, so a
    /// negative-amount `step` would hit that fallback and push the *disabled*
    /// colour further from the background than the enabled one: the unusable
    /// row would stand out more than the usable ones.
    #[test]
    fn toward_never_reverses_the_way_a_step_would() {
        let dark = ColorPalette::dark();
        let dim = ColorPalette::toward(dark.foreground, dark.background, 0.5);
        assert!(
            luminance(dim) < luminance(dark.foreground),
            "a near-white foreground must dim downward toward a near-black page"
        );
    }

    /// Interpolation cannot overshoot, which is the second reason it is not a
    /// per-channel add. Even at 1.0 the result is exactly the surface, and
    /// beyond that it is clamped rather than continuing past it.
    #[test]
    fn toward_lands_on_the_surface_and_goes_no_further() {
        let p = ColorPalette::dark();
        // Compared within a tolerance rather than bit-for-bit: `a + (b - a) * 1.0`
        // is `b` mathematically but lands one ulp away in f32, and a test that
        // demanded exactness here would be asserting on the rounding rather
        // than on the property.
        let near = |a: Color, b: Color, what: &str| {
            let (a, b) = (a.to_srgba().to_vec3(), b.to_srgba().to_vec3());
            assert!((a - b).abs().max_element() < 1e-6, "{what}: {a:?} vs {b:?}");
        };
        near(
            ColorPalette::toward(p.foreground, p.background, 1.0),
            p.background,
            "all the way is the surface",
        );
        near(
            ColorPalette::toward(p.foreground, p.background, 2.0),
            p.background,
            "an out-of-range amount must clamp, not invert past the surface",
        );
    }

    /// Zero is the identity, matching `step`'s own contract.
    #[test]
    fn toward_by_nothing_changes_nothing() {
        let p = ColorPalette::light();
        let moved = ColorPalette::toward(p.foreground, p.background, 0.0);
        assert_eq!(
            moved.to_srgba().to_vec3(),
            p.foreground.to_srgba().to_vec3()
        );
    }

    /// A dimmed colour keeps its own alpha rather than drifting toward the
    /// surface's. Fading alpha instead would let whatever sits behind the
    /// surface bleed through, and two overlapping dimmed things would compound.
    #[test]
    fn toward_preserves_the_base_alpha() {
        let base = Color::srgba(1.0, 1.0, 1.0, 0.5);
        let surface = Color::srgba(0.0, 0.0, 0.0, 1.0);
        assert_eq!(
            ColorPalette::toward(base, surface, 0.5).to_srgba().alpha,
            0.5
        );
    }
}
