//! When a command applies, and who wins when several do.
//!
//! Two commands may share a chord. `Escape` in a code editor means close the
//! completion popup, else collapse the multi-cursor, else close the find bar,
//! else leave the terminal — four handlers, one key, all reachable from the
//! same focused widget. What separates them is *application state*, not focus,
//! which is why tempera discriminates with conditions rather than with a focus
//! hierarchy.
//!
//! A condition is an ordinary Bevy run condition: any read-only system
//! returning `bool`. That buys `.and()`, `.or()` and `not()` for free, plus
//! `Res<T>` and `Query` injection, and means the application writes plain
//! typed systems instead of strings in a bespoke expression language:
//!
//! ```
//! use bevy::prelude::*;
//! use tempera_input::condition::When;
//!
//! #[derive(Resource, Default)]
//! struct Transport { playing: bool }
//! #[derive(Resource, Default)]
//! struct Selection(Vec<Entity>);
//!
//! fn is_playing(t: Res<Transport>) -> bool { t.playing }
//! fn has_selection(s: Res<Selection>) -> bool { !s.0.is_empty() }
//!
//! let mut world = World::new();
//! world.init_resource::<Transport>();
//! world.init_resource::<Selection>();
//!
//! let mut when = When::new(is_playing.and(not(has_selection)));
//! when.initialize(&mut world);
//! assert!(!when.eval(&world));
//!
//! world.resource_mut::<Transport>().playing = true;
//! assert!(when.eval(&world), "playing, nothing selected");
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

/// Gate on application state: this applies only when the condition holds.
///
/// An entity with no `When` is unconditional.
///
/// # Not only for commands
///
/// A command fires when its chord is pressed *and* its `When` passes. A
/// context-menu row is collected when its `When` passes. An inspector section
/// shows when its `When` passes. Same question — "does this apply right now?"
/// — so it is one type, sitting on whatever entity is asking.
///
/// `context_menu::VisibleWhen` was an independent second implementation of
/// exactly this, down to the field names and the errors-are-false rule. It is
/// now an alias.
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
                "[tempera-input] condition evaluated before initialize(); \
                 treating as false"
            );
            return false;
        }
        match self.condition.run_readonly((), world) {
            Ok(passed) => passed,
            Err(e) => {
                error!("[tempera-input] condition failed to run: {e}; treating as false");
                false
            }
        }
    }
}

/// Evaluate the `When` on `entity`, if it has one. No gate means `true`.
///
/// # Why the component is detached and put back
///
/// A condition is a `System`: running it needs `&mut` on the condition and
/// `&World` at the same moment, and those two borrows cannot both come from
/// one world. Taking the component out for the duration makes them disjoint.
///
/// Both callers arrived at this dance independently and wrote it twice. It
/// belongs with the type — the borrow problem is a property of `When`, not of
/// whoever is asking.
///
/// Initialization happens here too, so a gate added at runtime works without
/// its owner remembering to prime it.
pub fn passes(world: &mut World, entity: Entity) -> bool {
    let Some(mut gate) = world.entity_mut(entity).take::<When>() else {
        return true;
    };
    gate.initialize(world);
    let passed = gate.eval(world);
    world.entity_mut(entity).insert(gate);
    passed
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
    #[test]
    fn an_entity_with_no_gate_passes() {
        // Absence means unconditional — a menu row or command that never
        // declared a `When` must not be filtered out.
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        assert!(passes(&mut world, entity));
    }

    #[test]
    fn passes_evaluates_a_gate_in_place_and_leaves_it_attached() {
        // The component is detached to satisfy the borrow checker and put
        // back. Losing it would silently make the entity unconditional from
        // the second evaluation onward.
        let mut world = World::new();
        world.init_resource::<Playing>();
        let entity = world.spawn(When::new(is_playing)).id();

        assert!(!passes(&mut world, entity), "starts false");
        assert!(
            world.entity(entity).contains::<When>(),
            "the gate must survive its own evaluation"
        );

        world.resource_mut::<Playing>().0 = true;
        assert!(passes(&mut world, entity), "and still works after");
    }

    #[test]
    fn passes_initializes_a_gate_added_after_the_fact() {
        // A gate inserted at runtime has never been primed. `passes` does it,
        // so its owner does not have to remember.
        let mut world = World::new();
        world.init_resource::<Playing>();
        world.resource_mut::<Playing>().0 = true;
        let entity = world.spawn_empty().id();

        world.entity_mut(entity).insert(When::new(is_playing));

        assert!(
            passes(&mut world, entity),
            "an uninitialized gate must be primed, not treated as false"
        );
    }
}
