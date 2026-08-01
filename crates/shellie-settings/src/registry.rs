//! Finding a tab body by id.
//!
//! Content parents itself into a tab body; this crate never pushes content
//! into one. That inversion is what lets a tab's content live in a crate
//! this one has never heard of — the same contract `shellie-dock` gives a
//! pane, and the reason an unrecognised id is *ignorable* rather than
//! wrong.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::tab::{TabBody, TabId};

/// Convenience access to tab bodies for content that parents itself in.
///
/// ```ignore
/// fn spawn_audio_tab(mut commands: Commands, bodies: TabBodies, mine: Query<(), With<MyTab>>) {
///     if !mine.is_empty() { return; }              // spawn once
///     let Some(body) = bodies.get("audio") else { return };  // poll until it exists
///     commands.spawn((MyTab, ChildOf(body)));
/// }
/// ```
#[derive(SystemParam)]
pub struct TabBodies<'w, 's> {
    bodies: Query<'w, 's, (Entity, &'static TabId), With<TabBody>>,
}

impl TabBodies<'_, '_> {
    /// The body entity for `id`, if that tab exists yet.
    ///
    /// `None` covers both "not built yet" and "no such tab", deliberately:
    /// a panel polls until its body appears, so the two must be the same
    /// answer at this level.
    pub fn get(&self, id: &str) -> Option<Entity> {
        self.bodies
            .iter()
            .find(|(_, body_id)| body_id.as_str() == id)
            .map(|(entity, _)| entity)
    }

    /// Whether `id` names a live tab body.
    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// Every tab body, in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Entity)> {
        self.bodies.iter().map(|(e, id)| (id.as_str(), e))
    }
}

/// Run condition: true once `id` names a live tab body.
///
/// Lets a panel write `.run_if(tab_exists("audio"))` instead of opening
/// every system with the same early return.
pub fn tab_exists(id: &'static str) -> impl Fn(Query<&TabId, With<TabBody>>) -> bool + Clone {
    move |bodies| bodies.iter().any(|body_id| body_id.as_str() == id)
}
