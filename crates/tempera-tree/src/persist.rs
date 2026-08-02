//! Saving and restoring expansion.
//!
//! [`TreeState`] is the persisted type — there is no second save-shape to keep
//! in step with the live one. This crate picks no file format:
//! [`capture`](TreeState::capture) hands back a `Serialize` value and
//! [`apply`](TreeState::apply) takes one; where it goes is the host's business.
//!
//! # Unknown ids are kept
//!
//! `tempera-input` treats an unknown saved keybind as ignorable rather than as
//! a corrupt file, and `tempera-dock` keeps a saved pane the build no longer
//! declares. The same doctrine applies here, **and harder**.
//!
//! For the dock, unknown-at-load means the app dropped a panel — rare, and a
//! release-boundary event. Here the ids come from filesystem paths and plugin
//! scans that finish *after* the state loads. Unknown-at-load is not an edge
//! case, it is the steady state for the first few hundred milliseconds of every
//! launch. Pruning on that basis would collapse every folder the user had open,
//! on every restart, on the strength of a race.
//!
//! So nothing is ever dropped automatically. A host that genuinely knows an id
//! is retired calls [`TreeState::retain`](crate::TreeState::retain).
//!
//! # Version
//!
//! A file whose `version` this build does not understand is refused, and the
//! live state is left alone — losing which folders were open is a smaller harm
//! than adopting a shape this build will misread.

use bevy::prelude::*;

use crate::item::TreeId;
use crate::state::{FORMAT_VERSION, TreeState};

impl TreeState {
    /// Read the live state for the tree `id` out of the world.
    ///
    /// `None` when no view for that id exists yet, so a save triggered during
    /// startup writes nothing rather than writing an empty state over a file
    /// that had the user's real layout in it.
    pub fn capture(world: &mut World, id: &TreeId) -> Option<Self> {
        let mut views = world.query::<(&TreeId, &TreeState)>();
        views
            .iter(world)
            .find(|(view_id, _)| *view_id == id)
            .map(|(_, state)| state.clone())
    }

    /// Adopt a saved state onto the view entity carrying `id`.
    ///
    /// Refuses a file from a future format and keeps the live state. Missing
    /// views are a no-op with a warning — the tree may simply not be built yet.
    pub fn apply(&self, world: &mut World, id: &TreeId) {
        if self.version > FORMAT_VERSION {
            error!(
                "[tempera-tree] saved state for {:?} is format {} but this build understands \
                 at most {}; keeping the current expansion",
                id.as_str(),
                self.version,
                FORMAT_VERSION,
            );
            return;
        }

        let mut views = world.query::<(Entity, &TreeId)>();
        let targets: Vec<Entity> = views
            .iter(world)
            .filter(|(_, view_id)| *view_id == id)
            .map(|(entity, _)| entity)
            .collect();

        if targets.is_empty() {
            warn!(
                "[tempera-tree] no tree view carries the id {:?}; the saved expansion was not \
                 applied",
                id.as_str(),
            );
            return;
        }

        for entity in targets {
            world.entity_mut(entity).insert(self.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::GroupId;

    fn g(s: &str) -> GroupId {
        GroupId(s.to_string())
    }

    fn world_with_view(id: &str) -> (World, Entity) {
        let mut world = World::new();
        let entity = world.spawn((TreeId(id.to_string()), TreeState::new())).id();
        (world, entity)
    }

    #[test]
    fn capture_before_a_view_exists_is_none() {
        // A save during startup must not write an empty state over a good file.
        let mut world = World::new();
        assert!(TreeState::capture(&mut world, &TreeId("browser".into())).is_none());
    }

    #[test]
    fn capture_reads_only_the_named_tree() {
        let (mut world, browser) = world_with_view("browser");
        world.spawn((TreeId("outline".to_string()), TreeState::new()));

        let mut state = world.get_mut::<TreeState>(browser).unwrap();
        state.set_open(&g("src"), true, false);

        let captured = TreeState::capture(&mut world, &TreeId("browser".into())).unwrap();
        assert!(captured.is_open(&g("src"), false));

        let other = TreeState::capture(&mut world, &TreeId("outline".into())).unwrap();
        assert!(other.is_empty(), "the second tree has its own expansion");
    }

    #[test]
    fn apply_puts_the_saved_state_on_the_view() {
        let (mut world, view) = world_with_view("browser");

        let mut saved = TreeState::new();
        saved.set_open(&g("src"), true, false);
        saved.apply(&mut world, &TreeId("browser".into()));

        assert!(
            world
                .get::<TreeState>(view)
                .unwrap()
                .is_open(&g("src"), false)
        );
    }

    #[test]
    fn an_unknown_saved_id_is_kept() {
        // The load-bearing tolerance test. Group ids arrive from scans that
        // finish after the state loads, so unknown-at-load is the steady state.
        // Pruning here would collapse the user's folders on every restart.
        let (mut world, view) = world_with_view("browser");

        let mut saved = TreeState::new();
        saved.set_open(&g("not-scanned-yet"), true, false);
        saved.apply(&mut world, &TreeId("browser".into()));

        let live = world.get::<TreeState>(view).unwrap();
        assert!(live.is_open(&g("not-scanned-yet"), false));
        assert_eq!(live.len(), 1);
    }

    #[test]
    fn a_future_format_is_refused_and_the_live_state_survives() {
        let (mut world, view) = world_with_view("browser");
        {
            let mut live = world.get_mut::<TreeState>(view).unwrap();
            live.set_open(&g("mine"), true, false);
        }

        let mut saved = TreeState::new();
        saved.version = FORMAT_VERSION + 1;
        saved.set_open(&g("theirs"), true, false);
        saved.apply(&mut world, &TreeId("browser".into()));

        let live = world.get::<TreeState>(view).unwrap();
        assert!(live.is_open(&g("mine"), false), "live expansion untouched");
        assert!(!live.is_open(&g("theirs"), false));
    }

    #[test]
    fn applying_to_a_missing_view_is_not_fatal() {
        let mut world = World::new();
        TreeState::new().apply(&mut world, &TreeId("browser".into()));
    }

    #[test]
    fn the_state_survives_a_serde_round_trip() {
        let mut state = TreeState::new();
        state.set_open(&g("src"), true, false);
        state.set_open(&g("target"), false, true);

        let text = ron::to_string(&state).expect("serialize");
        let back: TreeState = ron::from_str(&text).expect("deserialize");

        assert_eq!(back.version, FORMAT_VERSION);
        assert!(back.is_open(&g("src"), false));
        assert!(!back.is_open(&g("target"), true));
        assert_eq!(back.len(), 2);
    }

    #[test]
    fn a_saved_file_is_byte_stable() {
        // `BTreeSet` rather than `HashSet`: a diff in version control should
        // mean the user changed something, not that a hash seed moved.
        let mut a = TreeState::new();
        a.set_open(&g("z"), true, false);
        a.set_open(&g("a"), true, false);

        let mut b = TreeState::new();
        b.set_open(&g("a"), true, false);
        b.set_open(&g("z"), true, false);

        assert_eq!(ron::to_string(&a).unwrap(), ron::to_string(&b).unwrap());
    }
}
