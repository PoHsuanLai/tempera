//! Shared animation primitives — a damped spring and a timed ease-tween,
//! both generic over what they carry.
//!
//! Pick by whether the motion is *physical* or *scheduled*:
//!
//! * [`Spring`] — settles naturally with no fixed duration, so it reads
//!   smoothly however often the target moves. Right for widget
//!   micro-motion (switch thumb, checkbox glyph, toast slide) and for
//!   anything that should feel alive under repeated input.
//! * [`EaseTween`] — a fixed-duration eased jump. Right when the user
//!   expects "go there, take ~125 ms, stop": programmatic scroll jumps,
//!   auto-follow, go-to-selection.
//!
//! # Why the carrier is generic
//!
//! Both types animate anything implementing [`Lerpable`] — `f32` out of
//! the box, and a consumer's own type by writing four small methods. A
//! layered-look struct or a layout-params bundle springs as one unit
//! that way, with a single velocity, instead of N independent scalar
//! springs that can drift out of phase with each other.
//!
//! # Why the spring takes its parameters per step
//!
//! Stiffness and damping are properties of *the motion being asked for*,
//! not of the value being moved. Storing them on the struct makes every
//! spring carry two floats that its own driving system already knows,
//! and invites them being mutated to something the caller never intended.
//! Passing them to [`Spring::step`] keeps the caller honest about what
//! feel it is asking for, and keeps the component to three fields.

use bevy::prelude::*;
use bevy_math::curve::{Curve, EaseFunction, EasingCurve};

/// The componentwise vector arithmetic a spring or tween needs from
/// whatever it carries.
///
/// Deliberately **not** `bevy_math::VectorSpace` — that requires `Mul`,
/// `Div`, `Neg` and a `ZERO` const, none of which an animation needs,
/// and it cannot be implemented for a struct of unrelated fields
/// (a look with a colour, a width and an opacity) without inventing
/// arithmetic those fields do not have. Four methods is the whole
/// surface; anything a consumer can lerp, it can animate.
pub trait Lerpable: Copy {
    fn lerp_add(self, delta: Self) -> Self;
    fn lerp_sub(self, other: Self) -> Self;
    fn lerp_scale(self, k: f32) -> Self;
    /// Whether this value is close enough to zero to stop integrating.
    /// Implementors decide their own epsilon — a colour and a pixel
    /// offset do not share one.
    fn approx_zero(self) -> bool;
}

/// Damped harmonic oscillator, generic over its carrier.
///
/// Drive it with [`Spring::step`] once per frame and read [`Spring::value`].
/// Write [`Spring::target`] to retarget — it does not restart anything,
/// so retargeting every frame (following a moving value) is fine.
#[derive(Component, Copy, Clone, Debug, Default)]
pub struct Spring<T: Lerpable> {
    pub value: T,
    pub velocity: T,
    pub target: T,
}

impl<T: Lerpable> Spring<T> {
    /// A spring sitting at `initial`, targeting `initial`, at rest.
    ///
    /// Seed with the widget's current state so it does not animate in
    /// from zero on spawn.
    pub fn new(initial: T) -> Self {
        Self {
            value: initial,
            velocity: initial.lerp_scale(0.0),
            target: initial,
        }
    }

    /// Semi-implicit (symplectic) Euler step toward [`Self::target`].
    ///
    /// `k` is stiffness (higher = faster), `damping` resists overshoot.
    /// Pairs in use: `(800, 30)` for a switch-style slide, `(2000, 55)`
    /// for a snappy glyph reveal, `(250, 25)` for a toast slide-in.
    ///
    /// `dt` is clamped to 1/60 s. Semi-implicit Euler is only
    /// conditionally stable: at these stiffnesses a long frame — a
    /// debugger pause, a hitch, a `dt` of a whole second — feeds back
    /// enough energy to diverge, and the widget flies off screen rather
    /// than arriving late. Clamping trades exactness across a hitch for
    /// never exploding, which is the right trade for UI motion.
    ///
    /// Snaps exactly to the target once both the offset and the velocity
    /// are negligible, so a settled spring stops mutating its component
    /// and stops waking change detection.
    pub fn step(&mut self, dt: f32, k: f32, damping: f32) {
        let dt = dt.min(1.0 / 60.0);
        let restoring = self.value.lerp_sub(self.target).lerp_scale(-k);
        let drag = self.velocity.lerp_scale(-damping);
        let accel = restoring.lerp_add(drag);
        self.velocity = self.velocity.lerp_add(accel.lerp_scale(dt));
        self.value = self.value.lerp_add(self.velocity.lerp_scale(dt));
        if self.value.lerp_sub(self.target).approx_zero() && self.velocity.approx_zero() {
            self.value = self.target;
            self.velocity = self.velocity.lerp_scale(0.0);
        }
    }

    /// Jump to `v` and come to rest there. Use for initial-state seeding
    /// and for "no animation, just be there" cases (project load, undo).
    pub fn snap_to(&mut self, v: T) {
        self.value = v;
        self.target = v;
        self.velocity = v.lerp_scale(0.0);
    }

    /// Whether the spring has arrived and stopped. Gate per-frame work
    /// on this so settled widgets cost nothing.
    pub fn settled(&self) -> bool {
        self.value.lerp_sub(self.target).approx_zero() && self.velocity.approx_zero()
    }
}

/// Animation length used by [`EaseTween::smooth`] — 125 ms, the
/// VS Code smooth-scroll feel.
pub const SMOOTH_DURATION_SECS: f32 = 0.125;

/// Small backdate so the first sampled frame already shows visible
/// motion, instead of the "stuck for one frame" look on short jumps.
const BACKDATE_SECS: f32 = 0.010;
const BACKDATE_DURATION: f32 = 0.010;

/// Fixed-duration eased tween, generic over its carrier.
///
/// Build at the current value, [`EaseTween::set_target`] to request a
/// jump, [`EaseTween::tick`] each frame, read [`EaseTween::value`].
/// Setting the same target twice is a no-op, so a caller that recomputes
/// its target every frame will not perpetually re-anchor the curve.
#[derive(Clone, Debug)]
pub struct EaseTween<T: Lerpable> {
    /// Current sampled value.
    pub value: T,
    /// Where the value should land.
    pub target: T,
    /// Total animation length in seconds. `0.0` is effectively instant.
    pub duration: f32,
    /// Curve sampled across the animation.
    pub easing: EaseFunction,
    /// In-flight animation state; `None` when settled.
    anim: Option<AnimState<T>>,
}

#[derive(Clone, Debug)]
struct AnimState<T: Lerpable> {
    from: T,
    to: T,
    elapsed: f32,
    duration: f32,
}

impl<T: Lerpable> EaseTween<T> {
    /// A settled tween at `initial`, nothing in flight.
    pub fn new(initial: T) -> Self {
        Self {
            value: initial,
            target: initial,
            duration: 0.0,
            easing: EaseFunction::CubicOut,
            anim: None,
        }
    }

    /// 125 ms cubic-out from `initial` — the scroll-jump preset.
    pub fn smooth(initial: T) -> Self {
        Self {
            duration: SMOOTH_DURATION_SECS,
            ..Self::new(initial)
        }
    }

    /// Retarget. Returns `true` when this actually started a new
    /// animation; an unchanged target returns `false` and leaves any
    /// in-flight curve alone.
    pub fn set_target(&mut self, target: T) -> bool {
        let target_changed = match &self.anim {
            Some(a) => !a.to.lerp_sub(target).approx_zero(),
            None => !self.value.lerp_sub(target).approx_zero(),
        };
        self.target = target;
        if !target_changed {
            return false;
        }
        self.anim = Some(AnimState {
            from: self.value,
            to: target,
            elapsed: BACKDATE_SECS,
            duration: (self.duration + BACKDATE_DURATION).max(0.001),
        });
        true
    }

    /// Jump to `v`, discarding any in-flight animation.
    pub fn snap_to(&mut self, v: T) {
        self.value = v;
        self.target = v;
        self.anim = None;
    }

    /// Advance by `dt` and update [`Self::value`]. Returns `true` while
    /// an animation is still in flight.
    pub fn tick(&mut self, dt: f32) -> bool {
        let Some(anim) = self.anim.as_mut() else {
            return false;
        };
        anim.elapsed += dt;
        if anim.elapsed >= anim.duration {
            self.value = anim.to;
            self.anim = None;
            return false;
        }
        let t = (anim.elapsed / anim.duration).clamp(0.0, 1.0);
        let eased = EasingCurve::new(0.0_f32, 1.0_f32, self.easing).sample_clamped(t);
        // value = from + (to - from) * eased
        let delta = anim.to.lerp_sub(anim.from).lerp_scale(eased);
        self.value = anim.from.lerp_add(delta);
        true
    }

    /// Whether nothing is in flight — the value equals the target.
    pub fn is_settled(&self) -> bool {
        self.anim.is_none()
    }
}

impl Lerpable for f32 {
    fn lerp_add(self, delta: Self) -> Self {
        self + delta
    }
    fn lerp_sub(self, other: Self) -> Self {
        self - other
    }
    fn lerp_scale(self, k: f32) -> Self {
        self * k
    }
    fn approx_zero(self) -> bool {
        self.abs() < 1.0e-3
    }
}

impl Lerpable for Vec2 {
    fn lerp_add(self, delta: Self) -> Self {
        self + delta
    }
    fn lerp_sub(self, other: Self) -> Self {
        self - other
    }
    fn lerp_scale(self, k: f32) -> Self {
        self * k
    }
    fn approx_zero(self) -> bool {
        self.x.abs() < 1.0e-3 && self.y.abs() < 1.0e-3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Spring ────────────────────────────────────────────────────────

    #[test]
    fn a_spring_arrives_within_400ms() {
        let mut s = Spring::<f32>::new(0.0);
        s.target = 1.0;
        let dt: f32 = 1.0 / 60.0;
        let steps = (0.4_f32 / dt).ceil() as usize;
        for _ in 0..steps {
            s.step(dt, 180.0, 22.0);
        }
        assert!((s.value - 1.0).abs() < 0.01, "value={}", s.value);
    }

    #[test]
    fn snapping_clears_velocity() {
        let mut s = Spring::<f32>::new(0.0);
        s.target = 1.0;
        s.step(1.0 / 60.0, 180.0, 22.0);
        assert!(s.velocity.abs() > 0.0);
        s.snap_to(0.5);
        assert_eq!(s.value, 0.5);
        assert_eq!(s.target, 0.5);
        assert_eq!(s.velocity, 0.0);
    }

    /// A one-second frame must not launch the widget into orbit.
    ///
    /// This is the test the previous scalar `Spring` failed: it
    /// integrated whatever `dt` it was handed, so a hitch at stiffness
    /// 2000 (the checkbox glyph) fed back more energy than the damping
    /// could remove. Without the clamp in `step`, this asserts on a
    /// value in the thousands.
    #[test]
    fn a_long_frame_does_not_explode() {
        let mut s = Spring::<f32>::new(0.0);
        s.target = 1.0;
        s.step(1.0, 180.0, 22.0);
        assert!(s.value.is_finite());
        assert!(s.value.abs() < 5.0, "value={}", s.value);
    }

    #[test]
    fn a_settled_spring_sits_exactly_on_its_target() {
        let mut s = Spring::<f32>::new(1.0);
        s.step(1.0 / 60.0, 180.0, 22.0);
        assert_eq!(s.value, 1.0);
        assert_eq!(s.velocity, 0.0);
        assert!(s.settled());
    }

    /// The generic carrier is the point of the type, so cover a
    /// non-scalar one. Both axes must arrive — a spring that integrated
    /// only `x` would pass every `f32` test above.
    #[test]
    fn a_vector_spring_arrives_on_both_axes() {
        let mut s = Spring::<Vec2>::new(Vec2::ZERO);
        s.target = Vec2::new(3.0, -7.0);
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            s.step(dt, 180.0, 22.0);
        }
        assert!(s.settled(), "value={:?}", s.value);
        assert!((s.value.x - 3.0).abs() < 0.01, "x={}", s.value.x);
        assert!((s.value.y + 7.0).abs() < 0.01, "y={}", s.value.y);
    }

    /// An arrived spring must land *exactly* on its target and stop dead.
    ///
    /// `Spring` is a `Component`, so a value still drifting in the last
    /// few decimal places marks it changed every frame — the widget never
    /// goes quiet and a `settled()` gate never latches.
    ///
    /// The parameters matter. At the switch's stiffness (800/30) the
    /// integration happens to reach 1.0 bit-exactly on its own, so that
    /// pair cannot tell the auto-snap from its absence — an earlier
    /// version of this test used it and passed with the snap deleted.
    /// The toast's softer 250/25 has a long tail: without the snap it
    /// sits at 1.0000162 still carrying −0.00068 of velocity, inside the
    /// settle band but not on the target.
    #[test]
    fn an_arrived_spring_lands_exactly_and_stops() {
        let mut s = Spring::<f32>::new(0.0);
        s.target = 1.0;
        for _ in 0..40 {
            s.step(1.0 / 60.0, 250.0, 25.0);
        }
        assert_eq!(s.value, 1.0, "did not land exactly on the target");
        assert_eq!(s.velocity, 0.0, "still carrying velocity after arrival");
        assert!(s.settled());

        // And it stays there.
        s.step(1.0 / 60.0, 250.0, 25.0);
        assert_eq!(s.value, 1.0);
    }

    // ── EaseTween ─────────────────────────────────────────────────────

    #[test]
    fn a_fresh_tween_sits_at_its_initial_value() {
        let t = EaseTween::<f32>::smooth(10.0);
        assert_eq!(t.value, 10.0);
        assert!(t.is_settled());
    }

    #[test]
    fn retargeting_starts_an_animation() {
        let mut t = EaseTween::<f32>::smooth(0.0);
        assert!(t.set_target(100.0));
        assert!(!t.is_settled());
        assert_eq!(t.value, 0.0);
    }

    #[test]
    fn retargeting_to_the_same_place_changes_nothing() {
        let mut t = EaseTween::<f32>::smooth(50.0);
        assert!(!t.set_target(50.0));
        assert!(t.is_settled());
    }

    #[test]
    fn a_tween_lands_exactly_on_its_target() {
        let mut t = EaseTween::<f32>::smooth(0.0);
        t.set_target(100.0);
        for _ in 0..30 {
            t.tick(1.0 / 60.0);
        }
        assert!(t.is_settled());
        assert_eq!(t.value, 100.0);
    }

    #[test]
    fn a_cubic_out_tween_never_goes_backwards() {
        let mut t = EaseTween::<f32>::smooth(0.0);
        t.set_target(100.0);
        let dt = 1.0 / 240.0;
        let mut last = t.value;
        for _ in 0..60 {
            t.tick(dt);
            assert!(t.value >= last - 1e-4, "regressed: {} → {}", last, t.value);
            last = t.value;
        }
    }

    #[test]
    fn snapping_a_tween_discards_the_animation() {
        let mut t = EaseTween::<f32>::smooth(0.0);
        t.set_target(100.0);
        t.tick(0.001);
        assert!(!t.is_settled());
        t.snap_to(42.0);
        assert!(t.is_settled());
        assert_eq!(t.value, 42.0);
        assert_eq!(t.target, 42.0);
    }

    #[test]
    fn retargeting_mid_flight_picks_the_new_destination() {
        let mut t = EaseTween::<f32>::smooth(0.0);
        t.set_target(100.0);
        let dt = 1.0 / 240.0;
        for _ in 0..15 {
            t.tick(dt);
        }
        let mid = t.value;
        assert!(mid > 0.0 && mid < 100.0);
        t.set_target(50.0);
        for _ in 0..60 {
            t.tick(dt);
        }
        assert_eq!(t.value, 50.0);
    }
}
