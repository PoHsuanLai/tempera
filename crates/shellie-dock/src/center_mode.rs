//! Swappable center surfaces.
//!
//! A shell usually has one pane whose *contents* change wholesale rather than
//! by scrolling: an IDE's editor versus its diff view, a graphics editor's
//! canvas versus its node graph, a DAW's timeline versus its piano roll. That
//! is a different thing from a pane, and this module is its vocabulary.
//!
//! It is **types and markers only** — no systems and no UI. Each contributing
//! crate registers itself as an entity, and whatever renders the switcher
//! queries those entities, so no closed list of modes lives in the shell:
//!
//! ```
//! use bevy::prelude::*;
//! use shellie_dock::center_mode::*;
//!
//! fn register(mut commands: Commands) {
//!     commands.spawn((
//!         CenterMode,
//!         CenterModeId("piano_roll"),
//!         CenterModeLabel("Piano Roll"),
//!         CenterModeOrder(30),
//!     ));
//! }
//!
//! # fn my_systems() {}
//! let mut app = App::new();
//! app.add_systems(Startup, register.in_set(CenterModeRegistration));
//! app.add_systems(Update, my_systems.run_if(is_active("piano_roll")));
//! ```
//!
//! # Showing and hiding is the mode's job
//!
//! This module says which mode is active; it does not hide anything. A mode
//! that owns a camera wants `Camera::is_active`, one that owns a node tree
//! wants `Display::None`, and one that owns neither just wants its systems to
//! stop running — [`is_active`] covers the last case and the other two are a
//! few lines in the mode's own crate. Centralising it here would mean picking
//! one mechanism for all three, and `Display::None` versus `Visibility::Hidden`
//! is not a detail: only the former collapses layout and zeroes the measured
//! size a viewport-tracking camera reads.
//!
//! # [`CenterContent`] works without a dock
//!
//! It is a plain marker, so a test harness or a standalone demo can spawn one
//! full-window node tagged with it and run a mode with no dock present at all.
//! Nothing here reaches for a pane.

use bevy::prelude::*;

/// Marker for an entity representing a registered center mode.
///
/// Inserting it attaches the metadata peers ([`CenterModeId`],
/// [`CenterModeLabel`], [`CenterModeOrder`]) at their defaults through Bevy's
/// required-components chain. Contributors override the ones they care about —
/// at minimum [`CenterModeId`], since the default empty string identifies
/// nothing.
#[derive(Component, Default)]
#[require(CenterModeId, CenterModeLabel, CenterModeOrder)]
pub struct CenterMode;

/// Stable id for a center mode.
///
/// `&'static str` rather than an owned string: a mode is registered by a crate
/// at compile time, and the id round-trips through session saves. An id no
/// crate registers simply sits unused — no mode's systems fire, and nothing
/// errors.
#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CenterModeId(pub &'static str);

/// Human-readable label for whatever renders the mode switcher.
#[derive(Component, Default, Clone)]
pub struct CenterModeLabel(pub &'static str);

/// Optional icon.
///
/// Deliberately outside [`CenterMode`]'s require chain: not every mode has an
/// icon, and there is no useful default `Handle<Image>` to invent for the ones
/// that do not.
#[derive(Component, Clone)]
pub struct CenterModeIcon(pub Handle<Image>);

/// Sort key for the mode switcher; lower goes first.
#[derive(Component, Default, Clone, Copy)]
pub struct CenterModeOrder(pub i32);

/// Which center mode is active.
///
/// `None` means none is selected — normally only during startup, before the
/// shell picks a default.
#[derive(Resource, Default, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActiveCenterMode(pub Option<&'static str>);

impl ActiveCenterMode {
    /// Whether `id` is the active mode.
    pub fn is(&self, id: &'static str) -> bool {
        self.0 == Some(id)
    }
}

/// Marker on the node center-mode content parents into.
///
/// Usually a child of whichever pane the host treats as its center, but that
/// is the host's choice — see the module docs on running without a dock.
#[derive(Component)]
pub struct CenterContent;

/// Run condition: true while `id` is the active mode.
///
/// Lets a mode's systems go inert when the user is looking at something else,
/// without the mode's crate knowing what else exists.
pub fn is_active(id: &'static str) -> impl FnMut(Option<Res<ActiveCenterMode>>) -> bool {
    move |m| matches!(m, Some(active) if active.0 == Some(id))
}

/// The set every mode's registration system belongs to.
///
/// Anything that reads the registered modes must order itself `.after` this,
/// or it will render a partial set on the first frame.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CenterModeRegistration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_center_mode_matches_by_id() {
        let active = ActiveCenterMode(Some("timeline"));
        assert!(active.is("timeline"));
        assert!(!active.is("spectral"));
        assert!(!ActiveCenterMode::default().is("timeline"));
    }

    #[test]
    fn registering_a_mode_attaches_the_metadata_peers() {
        let mut world = World::new();
        let entity = world.spawn(CenterMode).id();

        assert!(world.get::<CenterModeId>(entity).is_some());
        assert!(world.get::<CenterModeLabel>(entity).is_some());
        assert!(world.get::<CenterModeOrder>(entity).is_some());
        assert!(
            world.get::<CenterModeIcon>(entity).is_none(),
            "an icon has no sensible default and must stay opt-in"
        );
    }

    #[test]
    fn is_active_is_false_when_no_mode_resource_exists() {
        // Modes may register before the shell inserts the resource; they must
        // stay inert rather than all firing at once.
        let mut world = World::new();
        let mut condition = IntoSystem::into_system(is_active("timeline"));
        condition.initialize(&mut world);
        assert!(!condition.run((), &mut world).unwrap());

        world.insert_resource(ActiveCenterMode(Some("timeline")));
        assert!(condition.run((), &mut world).unwrap());
    }
}
