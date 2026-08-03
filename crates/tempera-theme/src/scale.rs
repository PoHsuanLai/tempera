//! The two-strand modular scale, and the length newtypes it produces.
//!
//! # Why a scale at all
//!
//! Perception is ratio-based: the smallest detectable difference is a
//! constant *fraction* of the stimulus, not a constant amount (Weber 1834,
//! Fechner 1860). 4px and 8px look obviously different; 100px and 104px look
//! identical. So a spacing scale is geometric, for the same reason audio is
//! measured in decibels.
//!
//! # Why two strands
//!
//! A single 2:1 scale from base 4 gives 4, 8, 16, 32 — and nothing between 8
//! and 16, which is where most UI spacing actually lives. Tightening the
//! ratio instead puts most steps below the just-noticeable difference, which
//! offers choices that are not really distinguishable.
//!
//! So the scale interleaves two strands, each a clean doubling:
//!
//! ```text
//! steps 0, 2, 4, 6:   4,  8, 16, 32     ← base × 2^k
//! steps 1, 3, 5, 7:   6, 12, 24, 48     ← base × 3/2 × 2^k
//! ```
//!
//! Twice the granularity without inventing a third ratio. Consecutive steps
//! alternate between 3:2 (a perfect fifth) and 4:3 (a perfect fourth), whose
//! product is exactly 2 — which is what makes +2 a doubling from *every*
//! position, and what keeps every value a whole number forever given an even
//! base. Tim Brown documents double-stranded scales in "More Meaningful
//! Typography" (*A List Apart*, 2011).

use super::base::{Base, Step};

/// A distance between things — padding, margin, the gap in a flex row.
///
/// # No `Add`
///
/// Two gaps stacked give a *length*, not a gap: the scale is not closed
/// under addition (from base 4, 22 of 36 pairwise sums fall off it — 4 + 6
/// is 10, which is not a scale member). An `Add` impl would hand back
/// something typed as if it were on the scale when it is not, and the whole
/// point of the type is that being on the scale is what it asserts.
///
/// Same reasoning as `Beat + Beat` in tutti: the operation is arithmetically
/// fine and semantically meaningless.
///
/// # No conversion to [`ControlHeight`]
///
/// Deliberately absent. Heights answer to hit targets and to alignment with
/// neighbouring controls; gaps answer to perceptual spacing. They are
/// different constraints that happen to be denominated in the same unit,
/// and letting one silently become the other is how a title-bar height ends
/// up hand-copied into a downstream crate as a bare `44.0`.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
pub struct Gap(f32);

/// A corner radius.
///
/// Carries the one operation in this module with a proof —
/// [`Self::concentric_inner`].
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
pub struct Radius(f32);

/// The height of an interactive control — a button, an input, a menu row.
///
/// **Declared, not generated.** Of the heights tempera uses today (26, 28,
/// 32, 36, 44) *none* is a member of the spacing scale, and no published
/// design system derives control heights either; every one declares them.
/// They answer to hit targets (WCAG 2.5.8 AA is 24×24 CSS px) and to
/// aligning with whatever sits beside them.
///
/// The direction that works is `padding = (height − line_height) / 2` —
/// height declared from the grid, padding solved. The inverse
/// (`height = line_height + 2·padding`) lets content dictate height, which
/// drifts controls off the grid until they stop aligning with each other.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
pub struct ControlHeight(f32);

/// A text size in logical pixels.
///
/// **Curated, not generated.** Type must survive hinting, x-height and
/// optical size, none of which a ratio models — which is why every major
/// design system is algorithmic in spacing and hand-picked in type. Note
/// also that the 4px grid is near-universally applied to *line-height*, not
/// to font-size: all 15 Material 3 line-heights divide by 4, while the sizes
/// (57/45/22/14/11) do not.
///
/// It is a newtype here for the same reason the others are — so it cannot be
/// passed where a [`Gap`] belongs — not because it comes from a rule.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
pub struct FontSize(f32);

macro_rules! length_newtype {
    ($($t:ty),+ $(,)?) => {$(
        impl $t {
            /// Wrap a raw pixel value.
            #[must_use]
            #[inline]
            pub const fn px(v: f32) -> Self {
                Self(v)
            }

            /// The value in logical pixels.
            #[must_use]
            #[inline]
            pub const fn get(self) -> f32 {
                self.0
            }
        }

        impl From<$t> for bevy::ui::Val {
            #[inline]
            fn from(v: $t) -> Self {
                bevy::ui::Val::Px(v.0)
            }
        }
    )+};
}

length_newtype!(Gap, Radius, ControlHeight, FontSize);

impl Radius {
    /// The radius a child should use to sit concentrically inside this one,
    /// given the padding between them.
    ///
    /// # The proof
    ///
    /// A rounded rectangle is the Minkowski sum of a sharp rectangle with a
    /// disc of radius *r*. Offsetting the boundary inward by *d* shrinks
    /// that disc to *r − d* and **leaves the arc centres fixed**, so the
    /// inner and outer curves stay exactly *d* apart the whole way around —
    /// which is what "concentric" means and what the eye is actually
    /// reading.
    ///
    /// When *d > r* the true inward offset really is a sharp corner, so
    /// clamping at zero is the correct answer rather than a fudge.
    ///
    /// Exact for circular arcs; a very good approximation for squircles,
    /// which are not closed under offsetting. Apple ships the same rule as
    /// `ConcentricRectangle` (WWDC25).
    ///
    /// This replaces the guess-twice pattern: today `dialog` declares a card
    /// radius of 12 and its close button separately declares 4, two numbers
    /// nobody reconciled. With this the child asks its parent.
    #[must_use]
    #[inline]
    pub fn concentric_inner(self, padding: Gap) -> Radius {
        Radius((self.0 - padding.0).max(0.0))
    }
}

/// The generated spacing scale for one [`Base`].
///
/// Values are computed, never stored per-step, so there is no way to assign
/// one and break proportionality — the reason [`Self::at`] is a method
/// rather than the scale being a struct of public fields.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Scale {
    base: Base,
}

impl Scale {
    /// A scale generated from `base`.
    #[must_use]
    #[inline]
    pub const fn new(base: Base) -> Scale {
        Scale { base }
    }

    /// The base this scale was generated from.
    #[must_use]
    #[inline]
    pub const fn base(self) -> Base {
        self.base
    }

    /// The gap at a given step.
    ///
    /// `at(n) = strand(n mod 2) × 2^(n div 2)`, with `strand(0) = base` and
    /// `strand(1) = base × 3/2`, using floored division so negative steps
    /// continue the same interleaving downward.
    #[must_use]
    pub fn at(self, step: Step) -> Gap {
        let n = i32::from(step.index());
        // Floored (not truncated) division and a non-negative remainder, so
        // step −1 is half of step 1 rather than restarting the pattern at
        // zero. Rust's `%` truncates toward zero, hence `rem_euclid`.
        let octave = n.div_euclid(2);
        let strand = n.rem_euclid(2);
        let root = if strand == 0 {
            self.base.get()
        } else {
            self.base.get() * 1.5
        };
        Gap(root * 2f32.powi(octave))
    }

    /// The radius at a given step.
    ///
    /// Radii are drawn from the same scale as gaps — a radius and the
    /// padding beside it are read together by the eye, so a radius that is
    /// off-scale reads as an error against its own padding.
    #[must_use]
    #[inline]
    pub fn radius_at(self, step: Step) -> Radius {
        Radius(self.at(step).get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every base the constructor accepts, so the properties below are
    /// checked over the whole input space rather than at one point.
    fn every_base() -> impl Iterator<Item = Base> {
        (1u8..=127).filter_map(|n| Base::new(n * 2))
    }

    /// The step range any real layout uses. Beyond this the values overflow
    /// screens long before they overflow `f32`.
    fn every_step() -> impl Iterator<Item = Step> {
        (-4i8..=8).map(Step::new)
    }

    #[test]
    fn an_octave_is_exactly_a_doubling() {
        // The property the two-strand construction exists to have: +2 is ×2
        // from *every* position, because the strand ratios 3:2 and 4:3
        // multiply to exactly 2. This is what makes `Step::octave_up`
        // lawful while `Step: Add` is not.
        for base in every_base() {
            let scale = Scale::new(base);
            for step in every_step() {
                let here = scale.at(step).get();
                let octave = scale.at(step.octave_up()).get();
                assert_eq!(
                    octave,
                    here * 2.0,
                    "base {}, step {}: {here} → {octave}",
                    base.px(),
                    step.index()
                );
            }
        }
    }

    #[test]
    fn every_valid_base_yields_whole_pixels() {
        // The reason `Base::new` refuses odd numbers. Checked from step 0
        // upward: below the base, halving legitimately produces fractions
        // (that is what a sub-base step *is*), so the guarantee is about the
        // range a scale is generated *into*, not below it.
        for base in every_base() {
            let scale = Scale::new(base);
            for step in (0i8..=8).map(Step::new) {
                let v = scale.at(step).get();
                assert_eq!(
                    v.fract(),
                    0.0,
                    "base {} step {} gave {v}, which blurs at fractional scaling",
                    base.px(),
                    step.index()
                );
            }
        }
    }

    #[test]
    fn an_odd_base_would_have_gone_fractional() {
        // `Base::new` refuses odd numbers; this is the consequence it is
        // refusing, checked directly rather than trusted. Without it the
        // constructor's invariant is a rule with no demonstration — the
        // failure mode the units ledger in tutti was written to prevent.
        let odd_strand = 3.0f32 * 1.5;
        assert_eq!(odd_strand, 4.5, "step 1 from base 3 is a half-pixel");
        assert_ne!(odd_strand.fract(), 0.0);
    }

    #[test]
    fn the_strands_interleave_as_documented() {
        // The concrete scale from the default base, spelled out — so a
        // change to the generator that preserves the ratios but shifts the
        // phase still fails.
        let scale = Scale::new(Base::FOUR);
        let got: Vec<f32> = (0i8..8).map(|n| scale.at(Step::new(n)).get()).collect();
        assert_eq!(got, vec![4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0]);
    }

    #[test]
    fn steps_below_the_base_continue_the_pattern() {
        // `%` truncates toward zero in Rust, which would restart the
        // interleaving at 0 and make step −1 equal to step 1. Euclidean
        // division is what keeps it monotonic across the origin.
        let scale = Scale::new(Base::FOUR);
        assert_eq!(scale.at(Step::new(-1)).get(), 3.0);
        assert_eq!(scale.at(Step::new(-2)).get(), 2.0);
        assert_eq!(scale.at(Step::new(-3)).get(), 1.5);

        let mut prev = f32::NEG_INFINITY;
        for step in every_step() {
            let v = scale.at(step).get();
            assert!(v > prev, "step {} broke monotonicity", step.index());
            prev = v;
        }
    }

    #[test]
    fn stepping_twice_is_not_stepping_by_two() {
        // The tested-and-false claim that keeps `Add` off `Step`. If this
        // ever passes, the scale has stopped being two-strand and the
        // omission should be revisited.
        let scale = Scale::new(Base::FOUR);
        let once_then_once = scale.at(Step::new(1)).get() * 1.5; // 6 → 9
        let straight_to_two = scale.at(Step::new(2)).get(); // 8
        assert_ne!(once_then_once, straight_to_two);
    }

    #[test]
    fn the_scale_is_not_closed_under_addition() {
        // The tested-and-false claim that keeps `Add` off `Gap`. Two gaps
        // stacked give a length, not a scale member.
        let scale = Scale::new(Base::FOUR);
        let members: Vec<f32> = (0i8..6).map(|n| scale.at(Step::new(n)).get()).collect();
        let off_scale = members
            .iter()
            .flat_map(|a| members.iter().map(move |b| a + b))
            .filter(|sum| !members.contains(sum))
            .count();
        assert!(
            off_scale > 0,
            "if every sum landed on the scale, `Gap: Add` would be defensible"
        );
    }

    #[test]
    fn nesting_twice_equals_nesting_once() {
        // Concentric offsetting composes, which is what lets a widget nest
        // three deep without anyone tracking a running total.
        let outer = Radius::px(12.0);
        let step = Gap(4.0);
        assert_eq!(
            outer.concentric_inner(step).concentric_inner(step),
            outer.concentric_inner(Gap(8.0))
        );
    }

    #[test]
    fn a_radius_never_goes_negative() {
        // When the padding exceeds the radius the true inward offset is a
        // sharp corner, so zero is the right answer rather than a clamp
        // papering over one.
        assert_eq!(Radius::px(4.0).concentric_inner(Gap(16.0)), Radius::px(0.0));
        assert_eq!(Radius::px(0.0).concentric_inner(Gap(4.0)), Radius::px(0.0));
        for r in 0u8..40 {
            for p in 0u8..40 {
                let inner = Radius::px(f32::from(r)).concentric_inner(Gap(f32::from(p)));
                assert!(inner.get() >= 0.0);
            }
        }
    }

    #[test]
    fn radii_come_off_the_same_scale_as_gaps() {
        let scale = Scale::new(Base::FOUR);
        for step in every_step() {
            assert_eq!(scale.radius_at(step).get(), scale.at(step).get());
        }
    }
}
