//! What a host puts on an entity to make it a row.
//!
//! Six components, all optional except the marker and the two that answer
//! "who is this and what is it called". A host's scanner adds them as it
//! discovers items; nothing here is authored by hand.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// This entity is considered for a row.
///
/// Without it an entity is invisible to [`visible_rows`](crate::visible_rows)
/// no matter what else it carries, so a host can park half-built items in the
/// world without them flickering into the list.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TreeItem;

/// Which tree this item belongs to.
///
/// An app with a file browser and an outline view runs two trees over one
/// world; this is what keeps their rows apart. A host with exactly one tree
/// still needs it — there is no "the tree" default, because two trees is the
/// case that goes wrong silently.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TreeId(pub String);

/// The item's name: what sorts it, and what a query matches against.
///
/// One string, deliberately. A host's own label is usually richer — an
/// extension, a count, a device state — and it keeps that component and reads
/// it off [`VisibleRow::item`](crate::VisibleRow::item) at render time. The
/// tree needs a name to order and to match, and nothing else.
#[derive(Component, Debug, Clone)]
pub struct TreeName(pub String);

/// Present ⇒ this item is a group: it can hold children and can be collapsed.
///
/// The id must be unique within its [`TreeId`]. A duplicate is a host bug this
/// crate reports rather than resolves — see
/// [`visible_rows`](crate::visible_rows).
///
/// `Serialize`/`Deserialize` because [`TreeState`](crate::TreeState) persists
/// these ids — it is the only part of a tree that survives a restart.
#[derive(Component, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GroupId(pub String);

/// The group this item sits inside. Absent ⇒ it is a root.
///
/// The parent is named by id rather than by `Entity` because the scanner that
/// discovers a child usually has the parent's *path* and not its entity — a
/// filesystem walk mints `GroupId("src/widgets")` from a prefix long before it
/// knows what entity carries it, and the two halves may not even be in the same
/// frame.
#[derive(Component, Debug, Clone)]
pub struct ParentGroup(pub GroupId);

/// This group starts open. Absent ⇒ it starts closed.
///
/// A *default*, not a state: [`TreeState`](crate::TreeState) records only where
/// the user has disagreed, so a host that later changes its mind about a
/// default changes it for every user who never touched that group. That is the
/// whole reason expansion is stored as two sets rather than one.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DefaultOpen;

impl TreeId {
    /// Borrow the id as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl GroupId {
    /// Borrow the id as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for TreeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for TreeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for GroupId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for GroupId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
