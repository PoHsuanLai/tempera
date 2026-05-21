//! Shared animation primitives.
//!
//! Tempera uses physics-based damped springs for widget micro-motion
//! (switch thumb, checkbox glyph). Springs settle naturally to their
//! target without a fixed duration, so they read smoothly regardless
//! of toggle frequency.
//!
//! Lifted, simplified, from `armas_basic::animation::SpringAnimation`.

use bevy::prelude::*;

/// 1-D damped-spring simulation. Pair with `Spring::stepper(s, d)` to
/// pick parameters, drive with [`Spring::update`] each frame, and read
/// the `value` field.
#[derive(Component, Debug, Clone, Copy)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Spring {
    /// New spring sitting at `initial`, targeting `target`, with the
    /// default firm-but-not-bouncy parameters.
    pub const fn new(initial: f32, target: f32) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
            target,
            stiffness: 800.0,
            damping: 30.0,
        }
    }

    /// Override stiffness + damping. Higher stiffness = faster
    /// movement; higher damping = less overshoot. armas pairs:
    /// `(800, 30)` for switch-style slides, `(2000, 55)` for snappy
    /// glyph reveals.
    #[must_use]
    pub const fn params(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }

    pub const fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Semi-implicit Euler step. Stable for the spring parameters used
    /// across tempera widgets.
    pub fn update(&mut self, dt: f32) {
        let spring_force = -self.stiffness * (self.value - self.target);
        let damping_force = -self.damping * self.velocity;
        let acceleration = spring_force + damping_force;
        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;
    }

    /// True when the spring is close enough to its target that further
    /// updates won't be visible. Use to gate redraws / change
    /// detection so settled widgets don't keep waking the system.
    pub fn settled(&self) -> bool {
        (self.value - self.target).abs() < 0.001 && self.velocity.abs() < 0.001
    }

    /// Snap immediately to `value`, target the same value, and clear
    /// velocity. Use for initial-state seeding so the widget doesn't
    /// animate in from 0 on spawn.
    pub fn snap_to(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }
}
