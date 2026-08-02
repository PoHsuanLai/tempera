//! Finding a pane by id.
//!
//! Content parents itself into a pane; the dock never pushes content into one.
//! That inversion is deliberate — it is what lets a panel live in a crate the
//! dock has never heard of — and it means the lookup happens often: a panel
//! that spawns once still polls until its pane exists.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::node::{Pane, PaneId};

/// `PaneId` → `Entity` for the live tree.
///
/// A panel *can* scan `Query<(Entity, &PaneId)>` and compare strings, and that
/// always works. This keeps the poll O(1) instead of O(panes) per panel per
/// frame, and gives the lookup one obvious name.
///
/// Rebuilt whenever the tree changes structurally.
#[derive(Resource, Debug, Clone, Default)]
pub struct PaneRegistry {
    by_id: HashMap<String, Entity>,
}

impl PaneRegistry {
    /// The entity for `id`, if that pane is in the current tree.
    pub fn get(&self, id: &str) -> Option<Entity> {
        self.by_id.get(id).copied()
    }

    /// Whether `id` names a pane in the current tree.
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// How many panes the tree has.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether the tree has no panes — true before the first build.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Every pane, in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Entity)> {
        self.by_id.iter().map(|(id, e)| (id.as_str(), *e))
    }

    pub(crate) fn clear(&mut self) {
        self.by_id.clear();
    }

    pub(crate) fn insert(&mut self, id: &PaneId, entity: Entity) {
        self.by_id.insert(id.0.clone(), entity);
    }
}

/// Run condition: true once `id` names a live pane.
///
/// Lets a panel write `.run_if(pane_exists("browser"))` instead of opening
/// every system with the same early-return. The same idiom `shellie-input`
/// uses for command conditions.
///
/// ```
/// use bevy::prelude::*;
/// use shellie_dock::pane_exists;
///
/// # fn spawn_my_panel() {}
/// let mut app = App::new();
/// app.add_systems(Update, spawn_my_panel.run_if(pane_exists("browser")));
/// ```
pub fn pane_exists(id: &'static str) -> impl Fn(Option<Res<PaneRegistry>>) -> bool + Clone {
    move |registry| registry.is_some_and(|r| r.contains(id))
}

/// Convenience access to panes for content that parents itself in.
#[derive(SystemParam)]
pub struct Panes<'w, 's> {
    registry: Option<Res<'w, PaneRegistry>>,
    /// Fallback scan, so a `Panes` still resolves in a world where the
    /// registry resource was never inserted (a bare test world, say).
    panes: Query<'w, 's, (Entity, &'static PaneId), With<Pane>>,
}

impl Panes<'_, '_> {
    /// The entity for `id`, if that pane exists.
    pub fn get(&self, id: &str) -> Option<Entity> {
        if let Some(registry) = &self.registry
            && let Some(entity) = registry.get(id)
        {
            return Some(entity);
        }
        self.panes
            .iter()
            .find(|(_, pane_id)| pane_id.as_str() == id)
            .map(|(entity, _)| entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registered_pane_is_found_by_id() {
        let mut registry = PaneRegistry::default();
        let entity = Entity::from_raw_u32(7).unwrap();
        registry.insert(&PaneId::from("browser"), entity);

        assert_eq!(registry.get("browser"), Some(entity));
        assert!(registry.contains("browser"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn an_unknown_id_is_absent_rather_than_an_error() {
        // Content polls until its pane appears, so "not yet" and "never" have
        // to be the same answer at this level.
        let registry = PaneRegistry::default();
        assert_eq!(registry.get("nope"), None);
        assert!(registry.is_empty());
    }

    #[test]
    fn clearing_drops_every_entry() {
        let mut registry = PaneRegistry::default();
        registry.insert(&PaneId::from("a"), Entity::from_raw_u32(1).unwrap());
        registry.insert(&PaneId::from("b"), Entity::from_raw_u32(2).unwrap());
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn re_inserting_an_id_repoints_it() {
        // A rebuild spawns fresh entities for the same ids; the registry must
        // end up pointing at the new tree, not the retired one.
        let mut registry = PaneRegistry::default();
        let old = Entity::from_raw_u32(1).unwrap();
        let new = Entity::from_raw_u32(2).unwrap();
        registry.insert(&PaneId::from("a"), old);
        registry.insert(&PaneId::from("a"), new);
        assert_eq!(registry.get("a"), Some(new));
        assert_eq!(registry.len(), 1);
    }
}
