//! When a command applies, and who wins when several do.
//!
//! Two commands may share a chord. `Escape` in a code editor means close the
//! completion popup, else collapse the multi-cursor, else close the find bar,
//! else leave the terminal — four handlers, one key, all reachable from the
//! same focused widget. What separates them is *application state*, not focus,
//! which is why shellie discriminates with conditions rather than with a focus
//! hierarchy.
//!
//! A condition is an ordinary Bevy run condition: any read-only system
//! returning `bool`. That buys `.and()`, `.or()` and `not()` for free, plus
//! `Res<T>` and `Query` injection, and means the application writes plain
//! typed systems instead of strings in a bespoke expression language:
//!
//! ```ignore
//! fn is_playing(t: Res<Transport>) -> bool { t.playing }
//! fn has_selection(s: Res<Selection>) -> bool { !s.is_empty() }
//!
//! When::new(is_playing.and(not(has_selection)))
//! ```
//!
//! # Evaluating outside the scheduler
//!
//! Conditions normally run as part of `run_if`. Dispatch needs them on demand,
//! which Bevy supports: a `BoxedCondition` is a `ReadOnlySystem<Out = bool>`,
//! and `ReadOnlySystem::run_readonly(&mut self, input, &World)` is public and
//! safe. Two obligations come with that:
//!
//! - **Initialize before first run.** `System::initialize` resolves the
//!   system's parameter state; skipping it panics. [`When::initialize`] does
//!   it at registration, where `&mut World` is available.
//! - **Prefer plain state reads.** A condition using `Changed<T>` sees a
//!   different tick window when run on demand than it would inside the
//!   schedule, because its `last_run` advances on our calls rather than the
//!   scheduler's. Read the state instead of its change flag.

use bevy::ecs::schedule::{BoxedCondition, SystemCondition};
use bevy::prelude::*;

/// Gate on application state: this command applies only when the condition
/// holds.
///
/// A command with no `When` is unconditional.
#[derive(Component)]
pub struct When {
    condition: BoxedCondition,
    initialized: bool,
}

impl When {
    /// Wrap a Bevy run condition.
    pub fn new<M>(condition: impl SystemCondition<M>) -> Self {
        Self {
            condition: Box::new(IntoSystem::into_system(condition)),
            initialized: false,
        }
    }

    /// Resolve the condition's system parameters. Idempotent.
    ///
    /// Must run before [`eval`](Self::eval); doing it at registration keeps
    /// the per-keypress path free of world-structure mutation.
    pub fn initialize(&mut self, world: &mut World) {
        if !self.initialized {
            self.condition.initialize(world);
            self.initialized = true;
        }
    }

    /// Evaluate against the current world.
    ///
    /// Returns `false` if the condition was never initialized or if it errored
    /// — an unevaluatable gate must not silently behave as "allow".
    pub fn eval(&mut self, world: &World) -> bool {
        if !self.initialized {
            error!(
                "[shellie-input] condition evaluated before initialize(); \
                 treating as false"
            );
            return false;
        }
        match self.condition.run_readonly((), world) {
            Ok(passed) => passed,
            Err(e) => {
                error!("[shellie-input] condition failed to run: {e}; treating as false");
                false
            }
        }
    }
}

/// Tie-break among commands whose conditions all pass. Higher wins.
///
/// Absent means zero. Most commands never need this — it matters only where
/// several genuinely-applicable handlers share a chord, and the order between
/// them is a real product decision rather than an accident of registration.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Priority(pub i32);

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Playing(bool);

    fn is_playing(p: Res<Playing>) -> bool {
        p.0
    }

    #[test]
    fn evaluates_against_live_world_state() {
        let mut world = World::new();
        world.init_resource::<Playing>();

        let mut when = When::new(is_playing);
        when.initialize(&mut world);

        assert!(!when.eval(&world), "starts false");
        world.resource_mut::<Playing>().0 = true;
        assert!(when.eval(&world), "follows the resource");
    }

    #[test]
    fn composes_with_the_standard_combinators() {
        let mut world = World::new();
        world.init_resource::<Playing>();

        let mut when = When::new(not(is_playing));
        when.initialize(&mut world);

        assert!(when.eval(&world));
        world.resource_mut::<Playing>().0 = true;
        assert!(!when.eval(&world));
    }

    #[test]
    fn initialize_is_idempotent() {
        let mut world = World::new();
        world.init_resource::<Playing>();
        world.resource_mut::<Playing>().0 = true;

        let mut when = When::new(is_playing);
        when.initialize(&mut world);
        when.initialize(&mut world);

        assert!(when.eval(&world));
    }

    #[test]
    fn an_uninitialized_condition_denies_rather_than_allows() {
        let world = World::new();
        let mut when = When::new(|| true);
        // Deliberately skipping initialize().
        assert!(
            !when.eval(&world),
            "a gate that cannot be evaluated must not open"
        );
    }

    #[test]
    fn priority_orders_and_defaults_to_zero() {
        assert!(Priority(30) > Priority(10));
        assert_eq!(Priority::default(), Priority(0));
    }
}
