//! Where the seams are, for chrome that wants to sit on one.
//!
//! A toggle that shows and hides a panel reads best when it sits *on* the edge
//! it controls rather than beside it. Doing that needs a number — where the
//! edge is, right now, in logical pixels — and the question is who owns it.
//!
//! # Declare and resolve, not measure and cache
//!
//! The obvious implementation is for the chrome to name a pane and read its
//! `ComputedNode`. That works and has two costs. The reader ends up naming a
//! specific pane, so a bar wanting to track a *different* panel is a code
//! change; and only things with a `ComputedNode` can be tracked at all, which
//! excludes the overlays this crate deliberately does not model as panes.
//!
//! So the direction is inverted. **Whoever owns an edge declares where it is;
//! whoever wants to sit on one reads by id.** [`declare_pane_boundaries`] does
//! that for panes; an overlay writes its own entry and needs nothing from here
//! but the resource.
//!
//! That is the same inversion [`PaneRegistry`](crate::PaneRegistry) makes for
//! entities, and it buys the same thing: a reader that does not know what it
//! is reading about.
//!
//! # Absence is the vanishing case, and it is the writer's to signal
//!
//! A hidden pane measures zero. A closed overlay has no edge at all. Chrome
//! anchored to either must not jump to `x = 0` — that is off-window, and
//! exactly when a user wants to click it to bring the thing back.
//!
//! The rule is one line in each direction: a **writer removes its entry** when
//! its edge does not exist, and a **reader keeps its last position** when the
//! entry is missing. Neither consults a visibility flag, and the policy is
//! stated once rather than re-derived at each reader.
//!
//! This is why [`declare_pane_boundaries`] removes on a zero measurement
//! rather than writing the zero: a stored zero is a lie that every reader then
//! has to special-case.
//!
//! # It is not a cache
//!
//! A second place a position lives is only safe if it cannot go stale. This
//! one is rewritten every frame from the live layout and removed the frame its
//! subject stops being laid out, so there is no state to invalidate — the
//! failure mode a cache has (a *removal* nobody notices) is the one case
//! handled explicitly.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::ui::ComputedNode;

use crate::node::{Pane, PaneId};

/// Where each named edge currently is, in logical pixels from the window's
/// left.
///
/// Panes are declared automatically under their [`PaneId`]; anything else —
/// a floating panel, a rail, a HUD — writes its own id.
///
/// # Reading
///
/// ```ignore
/// fn anchor(boundaries: Res<Boundaries>, mut node: Single<&mut Node, With<MyToggle>>) {
///     // Absent means "no such edge right now" — hold the last position.
///     let Some(x) = boundaries.get("browser") else { return };
///     node.left = Val::Px(x - TOGGLE_WIDTH / 2.0);
/// }
/// ```
#[derive(Resource, Debug, Clone, Default)]
pub struct Boundaries(HashMap<String, f32>);

impl Boundaries {
    /// Where `id`'s edge is, or `None` if it has none right now.
    ///
    /// `None` is the *normal* state for a hidden pane or a closed overlay, not
    /// an error. See the module docs for what a reader should do with it.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<f32> {
        self.0.get(id).copied()
    }

    /// Declare where `id`'s edge is.
    ///
    /// Call every frame the edge exists. Overwrites, because this is a
    /// position rather than a claim — two writers for one id is a bug in the
    /// caller and first-wins would only hide it.
    pub fn set(&mut self, id: impl Into<String>, x: f32) {
        self.0.insert(id.into(), x);
    }

    /// Withdraw `id`'s edge, because it no longer exists.
    ///
    /// The counterpart to [`set`](Self::set), and the half that is easy to
    /// forget: chrome anchored to a stale entry sits on an edge that is not
    /// there.
    pub fn remove(&mut self, id: &str) {
        self.0.remove(id);
    }

    /// Every declared edge.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f32)> {
        self.0.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// Whether anything is declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Declare every pane's trailing edge under its [`PaneId`].
///
/// The *trailing* edge — right in a row, bottom in a column — because that is
/// the seam a pane shares with what follows it, and the one a divider sits on.
/// A leading edge is the previous sibling's trailing edge, so declaring both
/// would be two names for one number.
///
/// A pane measuring zero is hidden or not yet laid out; its entry is
/// **removed** rather than set to zero. See the module docs.
pub fn declare_pane_boundaries(
    panes: Query<(&PaneId, &ComputedNode, &UiGlobalTransform), With<Pane>>,
    mut boundaries: ResMut<Boundaries>,
) {
    for (id, computed, transform) in &panes {
        let size = computed.size() * computed.inverse_scale_factor;
        if size.x <= 0.0 || size.y <= 0.0 {
            // Hidden, or not laid out yet. Withdraw rather than declare a zero
            // that every reader would have to special-case.
            boundaries.remove(id.as_str());
            continue;
        }
        // `UiGlobalTransform` is the node's *centre*, so the trailing edge is
        // half a width further on.
        let right = transform.translation.x * computed.inverse_scale_factor + size.x / 2.0;
        boundaries.set(id.as_str(), right);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_edge_reads_as_none_rather_than_zero() {
        // The distinction the whole design rests on: zero is a *position*, and
        // a reader that received it would put its chrome at the window's left
        // edge. `None` says "there is no edge", which is a different claim.
        let mut boundaries = Boundaries::default();
        assert_eq!(boundaries.get("browser"), None);

        boundaries.set("browser", 200.0);
        assert_eq!(boundaries.get("browser"), Some(200.0));

        boundaries.remove("browser");
        assert_eq!(
            boundaries.get("browser"),
            None,
            "a withdrawn edge must not linger; chrome would sit on a seam that \
             is not there"
        );
    }

    #[test]
    fn a_second_write_wins() {
        // A position, not a claim. Two writers for one id is a caller bug, and
        // first-wins would hide it behind chrome that lags one of them.
        let mut boundaries = Boundaries::default();
        boundaries.set("center", 100.0);
        boundaries.set("center", 400.0);
        assert_eq!(boundaries.get("center"), Some(400.0));
    }

    #[test]
    fn ids_are_independent() {
        let mut boundaries = Boundaries::default();
        boundaries.set("browser", 200.0);
        boundaries.set("inspector", 900.0);
        boundaries.remove("browser");
        assert_eq!(boundaries.get("inspector"), Some(900.0));
    }
}
