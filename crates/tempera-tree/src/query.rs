//! The query a host runs to reach [`visible_rows`].
//!
//! There is no plugin, and that is the honest shape: this crate has no systems,
//! no resources and no schedule, because it renders nothing. A host adds no
//! plugin — it calls [`visible_rows`] where it wants rows.
//!
//! What is here is [`TreeQuery`], the ergonomic half: collecting a world's
//! items into the borrowed slice [`visible_rows`] wants is mechanical, and
//! every host would otherwise write it identically.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::item::{DefaultOpen, GroupId, ParentGroup, TreeId, TreeItem, TreeName};
use crate::state::TreeState;
use crate::visible::{RowOrder, TreeNode, VisibleRow, groups_first_then_name, visible_rows_by};

/// Everything [`visible_rows`] needs, gathered from the world.
///
/// ```ignore
/// fn rebuild(tree: TreeQuery, views: Query<(&TreeId, &TreeState)>) {
///     for (id, state) in &views {
///         for row in tree.rows(id, state, "") { /* spawn a row */ }
///     }
/// }
/// ```
#[derive(SystemParam)]
pub struct TreeQuery<'w, 's> {
    items: Query<
        'w,
        's,
        (
            Entity,
            &'static TreeId,
            &'static TreeName,
            Option<&'static GroupId>,
            Option<&'static ParentGroup>,
            Has<DefaultOpen>,
        ),
        With<TreeItem>,
    >,
}

impl TreeQuery<'_, '_> {
    /// The visible rows of the tree `id`, given `state` and `query`.
    ///
    /// Siblings sort by [`groups_first_then_name`]. Use
    /// [`rows_by`](Self::rows_by) for any other order.
    pub fn rows(&self, id: &TreeId, state: &TreeState, query: &str) -> Vec<VisibleRow> {
        self.rows_by(id, state, query, &groups_first_then_name)
    }

    /// [`rows`](Self::rows), with the sibling order chosen by the caller.
    pub fn rows_by(
        &self,
        id: &TreeId,
        state: &TreeState,
        query: &str,
        order: RowOrder<'_>,
    ) -> Vec<VisibleRow> {
        let nodes: Vec<TreeNode<'_>> = self
            .items
            .iter()
            .filter(|(_, item_id, ..)| *item_id == id)
            .map(|(entity, _, name, group, parent, default_open)| TreeNode {
                entity,
                name: &name.0,
                group,
                parent: parent.map(|p| &p.0),
                default_open,
            })
            .collect();
        visible_rows_by(&nodes, state, query, order)
    }

    /// Whether any item belongs to the tree `id`.
    ///
    /// Cheaper than building the row list when a host only needs to know
    /// whether a scan has produced anything yet.
    pub fn is_empty(&self, id: &TreeId) -> bool {
        !self.items.iter().any(|(_, item_id, ..)| item_id == id)
    }
}
