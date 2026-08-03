//! The two things a scale is built from: a base, and an index into it.
//!
//! [`Base`] is the one number a whole layout is derived from. [`Step`] is a
//! position on the scale, not a length — see its docs for why it has no
//! arithmetic.

/// The unit a spacing scale is generated from, in logical pixels.
///
/// # Even bases only
///
/// The scale interleaves two strands, the second at ×3/2 of the first (see
/// [`super::scale`]). An odd base makes that strand land on a half-pixel at
/// the very first step — base 3 gives 4.5 — and a fractional pixel blurs at
/// non-integer display scaling, which is exactly the case a base exists to
/// survive.
///
/// So the constructor rejects it. The alternative is rounding, and rounding
/// compounds: every later step computes from the rounded value, so a scale
/// that rounds once is off by more at every octave. Better to refuse the
/// input than to silently return a scale that is not geometric.
///
/// ```
/// # use tempera::theme::Base;
/// assert!(Base::new(6).is_some());
/// assert!(Base::new(3).is_none()); // 3 × 3/2 = 4.5
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Base(u8);

impl Base {
    /// The near-universal UI grid unit, and tempera's default. 4 is the
    /// smallest integer that survives every Android density bucket
    /// (0.75× → 3, 1.5× → 6, 2× → 8, 3× → 12), which is why it is the
    /// value nearly every design system converged on.
    pub const FOUR: Base = Base(4);

    /// A coarser grid, for layouts that want fewer distinct sizes.
    pub const EIGHT: Base = Base(8);

    /// A base, if `px` is even and non-zero.
    ///
    /// Returns `None` for odd values — see the type docs. Zero is rejected
    /// because a scale generated from it is all zeros, which is not a
    /// coarser layout but an invisible one.
    #[must_use]
    pub const fn new(px: u8) -> Option<Base> {
        if px != 0 && px.is_multiple_of(2) {
            Some(Base(px))
        } else {
            None
        }
    }

    /// The base in logical pixels.
    #[must_use]
    #[inline]
    pub const fn px(self) -> u8 {
        self.0
    }

    /// The base as `f32`, for the arithmetic every generator does.
    #[must_use]
    #[inline]
    pub fn get(self) -> f32 {
        f32::from(self.0)
    }
}

impl Default for Base {
    fn default() -> Self {
        Self::FOUR
    }
}

/// A position on the spacing scale — an **index**, not a length.
///
/// Step 0 is the base, step 1 is ×3/2, step 2 is ×2, and so on. Negative
/// steps go below the base.
///
/// # It has no arithmetic, and that is not an oversight
///
/// `Step` gets [`Ord`] and named methods, never `Add`. Adding steps looks
/// like it should compose and does not: on the two-strand scale from base 4,
/// `at(1)` is 6 and `at(2)` is 8, but stepping once from 6 gives 9 — because
/// the ratio applied at each position depends on that position's *absolute*
/// parity, not on the distance travelled. So `a + b` would have no
/// well-defined meaning: the answer depends on where you started.
///
/// [`Self::octave_up`] is the operation that *is* well-defined, because +2
/// is exactly ×2 from every position (the strand ratios 3:2 and 4:3 multiply
/// to 2). It is a method rather than an operator so that the one lawful
/// move is the easy one to reach for.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Step(i8);

impl Step {
    /// The base itself.
    pub const BASE: Step = Step(0);

    /// A step at the given index.
    #[must_use]
    #[inline]
    pub const fn new(index: i8) -> Step {
        Step(index)
    }

    /// The raw index.
    #[must_use]
    #[inline]
    pub const fn index(self) -> i8 {
        self.0
    }

    /// The step exactly one doubling above this one.
    ///
    /// Provably ×2: the two strand ratios multiply to exactly 2, so +2 is a
    /// doubling from *any* starting position. This is the only relative move
    /// on the scale that holds everywhere, which is why it is the only one
    /// with a method. Saturates rather than wrapping.
    #[must_use]
    #[inline]
    pub const fn octave_up(self) -> Step {
        Step(self.0.saturating_add(2))
    }

    /// The step exactly one halving below this one.
    #[must_use]
    #[inline]
    pub const fn octave_down(self) -> Step {
        Step(self.0.saturating_sub(2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_odd_base_is_rejected() {
        // The invariant the whole scale rests on: an odd base puts the ×3/2
        // strand on a half-pixel at step 1.
        for odd in [1u8, 3, 5, 7, 9, 15, 255] {
            assert!(Base::new(odd).is_none(), "{odd} is odd and must be refused");
        }
        for even in [2u8, 4, 6, 8, 12, 16, 254] {
            assert!(Base::new(even).is_some(), "{even} is even and usable");
        }
    }

    #[test]
    fn a_zero_base_is_rejected() {
        // Zero is even, so the parity check alone would let it through, and
        // the resulting scale is all zeros — an invisible layout rather than
        // a coarse one.
        assert!(Base::new(0).is_none());
    }

    #[test]
    fn an_octave_is_two_steps() {
        assert_eq!(Step::BASE.octave_up(), Step::new(2));
        assert_eq!(Step::new(3).octave_down(), Step::new(1));
    }

    #[test]
    fn stepping_saturates_rather_than_wrapping() {
        // A wrap would turn a very large step into a very small one, which
        // reads as a layout collapsing rather than as an error.
        assert_eq!(Step::new(i8::MAX).octave_up(), Step::new(i8::MAX));
        assert_eq!(Step::new(i8::MIN).octave_down(), Step::new(i8::MIN));
    }

    #[test]
    fn steps_order_by_index() {
        assert!(Step::new(-1) < Step::BASE);
        assert!(Step::BASE < Step::new(1));
    }
}
