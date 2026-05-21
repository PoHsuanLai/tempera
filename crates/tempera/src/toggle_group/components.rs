use bevy::prelude::*;

/// Root marker on a tempera toggle group.
#[derive(Component, Default, Debug)]
pub struct ToggleGroup;

/// Selection mode for a toggle group.
///
/// Stored as a separate component (not a field on [`ToggleGroup`]) so
/// it can be queried independently and changed at runtime without
/// touching the marker.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToggleGroupKind {
    /// Exclusive — one item at a time. Group emits `ValueChange<Entity>`.
    #[default]
    Single,
    /// Independent — any subset can be selected. Items emit `ValueChange<bool>`.
    Multiple,
}

/// Marker on each child item in a toggle group. The paint system uses
/// this to find items belonging to the group's row.
#[derive(Component, Default, Debug)]
pub struct ToggleItem;
