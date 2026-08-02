//! Which groups are open.
//!
//! # Why two sets and not one
//!
//! A `HashSet` of open ids carries one bit per group, and the question needs
//! two: *is this group open*, and *did the user say so*. Without the second,
//! "absent from the set" has to mean both "defaults closed" and "the user
//! closed a group that defaults open" — which are different, and the difference
//! is what a default is for.
//!
//! The cost of collapsing them is concrete and was already being paid: dawai's
//! browser ran a string-keyed set that defaulted everything *closed*, then a
//! second six-field resource with hand-written match arms that defaulted
//! everything *open*, because the first could not express a default-open group.
//! Two mechanisms, opposite defaults, and a section was structurally just a
//! root group the whole time.
//!
//! So the state records **deviations from the declared default**:
//!
//! - in `opened` ⇒ open, whatever the default says
//! - in `closed` ⇒ closed, whatever the default says
//! - in neither ⇒ whatever [`DefaultOpen`](crate::DefaultOpen) says
//!
//! An id is never in both — [`toggle`](TreeState::toggle) clears the opposite
//! set — and agreeing with the default clears the deviation entirely. That
//! keeps the saved file a small diff, and it means a host that changes a
//! default later has it take effect for every user who never disagreed.
//!
//! # On the view, not in a resource
//!
//! [`TreeState`] is a `Component`. Two trees in one app have independent
//! expansion, and a single global resource makes that unrepresentable — a bug
//! that stays invisible right up until the second tree exists.

use std::collections::BTreeSet;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::item::GroupId;

/// Bump when the on-disk shape changes incompatibly.
pub const FORMAT_VERSION: u32 = 1;

/// Per-tree expansion, stored as deviations from the declared defaults.
///
/// `BTreeSet` rather than `HashSet` so a saved file has a stable order and two
/// saves of the same state produce the same bytes — a diff in version control
/// should mean the user changed something.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct TreeState {
    /// On-disk format version.
    pub version: u32,
    /// Groups the user opened that would otherwise be closed.
    opened: BTreeSet<GroupId>,
    /// Groups the user closed that would otherwise be open.
    closed: BTreeSet<GroupId>,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            opened: BTreeSet::new(),
            closed: BTreeSet::new(),
        }
    }
}

impl TreeState {
    /// Fresh state: every group sits at its declared default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Is `group` open, given what its declaration says by default?
    ///
    /// `default_open` is the presence of [`DefaultOpen`](crate::DefaultOpen) on
    /// the group's entity. It is a parameter rather than something looked up
    /// here so this stays a pure function of the state.
    pub fn is_open(&self, group: &GroupId, default_open: bool) -> bool {
        if self.opened.contains(group) {
            true
        } else if self.closed.contains(group) {
            false
        } else {
            default_open
        }
    }

    /// Set `group` open or closed, recording it only if it deviates.
    ///
    /// Setting a group to its own default clears the deviation rather than
    /// recording it, so the state never grows entries that say nothing.
    pub fn set_open(&mut self, group: &GroupId, open: bool, default_open: bool) {
        self.opened.remove(group);
        self.closed.remove(group);
        if open == default_open {
            return;
        }
        if open {
            self.opened.insert(group.clone());
        } else {
            self.closed.insert(group.clone());
        }
    }

    /// Flip `group`.
    pub fn toggle(&mut self, group: &GroupId, default_open: bool) {
        let now = self.is_open(group, default_open);
        self.set_open(group, !now, default_open);
    }

    /// Forget every deviation — every group returns to its declared default.
    pub fn reset(&mut self) {
        self.opened.clear();
        self.closed.clear();
    }

    /// Drop deviations for groups the host says are gone.
    ///
    /// The host's explicit prune. Nothing calls this automatically: see
    /// [`persist`](crate::persist) for why an unknown id is kept by default.
    pub fn retain(&mut self, keep: impl Fn(&GroupId) -> bool) {
        self.opened.retain(&keep);
        self.closed.retain(&keep);
    }

    /// Every group the state has an opinion about, in id order.
    ///
    /// For a host implementing `retain`, and for tests. The bool is the opinion.
    pub fn deviations(&self) -> impl Iterator<Item = (&GroupId, bool)> {
        self.opened
            .iter()
            .map(|g| (g, true))
            .chain(self.closed.iter().map(|g| (g, false)))
    }

    /// How many groups deviate from their default.
    pub fn len(&self) -> usize {
        self.opened.len() + self.closed.len()
    }

    /// Is every group at its declared default?
    pub fn is_empty(&self) -> bool {
        self.opened.is_empty() && self.closed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(s: &str) -> GroupId {
        GroupId(s.to_string())
    }

    #[test]
    fn an_untouched_group_follows_its_declaration() {
        let state = TreeState::new();
        assert!(state.is_open(&g("open-by-default"), true));
        assert!(!state.is_open(&g("shut-by-default"), false));
    }

    #[test]
    fn closing_a_default_open_group_is_recorded() {
        // The case a bare `HashSet` of open ids cannot express: absent from
        // that set would read as "closed", which is also what an untouched
        // default-closed group reads as.
        let mut state = TreeState::new();
        state.toggle(&g("src"), true);

        assert!(!state.is_open(&g("src"), true));
        assert_eq!(state.len(), 1, "the deviation is recorded");
        assert_eq!(state.deviations().collect::<Vec<_>>(), [(&g("src"), false)]);
    }

    #[test]
    fn opening_a_default_closed_group_is_recorded() {
        let mut state = TreeState::new();
        state.toggle(&g("target"), false);

        assert!(state.is_open(&g("target"), false));
        assert_eq!(
            state.deviations().collect::<Vec<_>>(),
            [(&g("target"), true)]
        );
    }

    #[test]
    fn returning_to_the_default_clears_the_deviation() {
        // Otherwise the file accumulates entries that say nothing, and a host
        // that changes a default later cannot reach users who never disagreed.
        let mut state = TreeState::new();
        state.toggle(&g("src"), true);
        assert_eq!(state.len(), 1);

        state.toggle(&g("src"), true);
        assert!(state.is_open(&g("src"), true));
        assert!(
            state.is_empty(),
            "agreeing with the default records nothing"
        );
    }

    #[test]
    fn a_group_is_never_in_both_sets() {
        let mut state = TreeState::new();
        state.set_open(&g("a"), true, false);
        state.set_open(&g("a"), false, true);

        assert_eq!(state.len(), 1);
        assert!(!state.is_open(&g("a"), true));
    }

    #[test]
    fn a_changed_default_reaches_users_who_never_disagreed() {
        // The point of storing deviations. `b` was never touched, so flipping
        // the declaration moves it; `a` was touched, so it holds its ground.
        let mut state = TreeState::new();
        state.toggle(&g("a"), false); // user opened a default-closed group

        assert!(state.is_open(&g("a"), true), "user's choice survives");
        assert!(
            state.is_open(&g("b"), true),
            "untouched group follows the new default"
        );
    }

    #[test]
    fn retain_drops_what_the_host_retires() {
        let mut state = TreeState::new();
        state.set_open(&g("live"), true, false);
        state.set_open(&g("gone"), true, false);

        state.retain(|id| id.as_str() == "live");
        assert_eq!(state.deviations().collect::<Vec<_>>(), [(&g("live"), true)]);
    }

    #[test]
    fn reset_returns_everything_to_its_default() {
        let mut state = TreeState::new();
        state.set_open(&g("a"), true, false);
        state.set_open(&g("b"), false, true);

        state.reset();
        assert!(state.is_empty());
        assert!(state.is_open(&g("b"), true));
    }
}
