//! Turning a declared [`DockLayout`] into entities.
//!
//! The build **reconciles** rather than re-spawns: a pane that survives a
//! layout change keeps its entity, and therefore keeps the content some other
//! crate parented into it. Rebuilding the world on every resize would work
//! exactly once and then lose every panel's state.
//!
//! Reconciliation is by [`PaneId`]. Panes present before and after keep their
//! frame; panes that disappear are despawned with their content; panes that
//! appear are spawned empty for content to find. Splits and dividers have no
//! identity of their own and are rebuilt freely — nothing parents into them.

use bevy::prelude::*;

use crate::node::{
    Axis, Divider, DockNode, DockRoot, Pane, PaneId, PaneMinSize, PaneSize, PaneVisibility, Split,
};
use crate::registry::PaneRegistry;
use crate::resize::{DividerStyle, on_divider_drag, on_divider_hover, on_divider_out};
use crate::tree::{DockLayout, DockTree};

/// When the dock materializes its tree.
///
/// A host that spawns content into panes should run after this, though content
/// that polls for its pane does not need to care.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DockBuildSet;

/// Set by a structural mutation to ask for a rebuild on the next run.
///
/// [`DockLayout`] change detection covers a host swapping the resource;
/// this covers the dock editing it in place, where `Changed` would also fire
/// but says nothing about *what* changed.
#[derive(Resource, Debug, Default)]
pub(crate) struct RebuildRequested(pub bool);

/// Build or reconcile the tree whenever the layout changes.
pub(crate) fn build_dock(
    mut commands: Commands,
    layout: Res<DockLayout>,
    mut rebuild: ResMut<RebuildRequested>,
    mut registry: ResMut<PaneRegistry>,
    style: Res<DividerStyle>,
    roots: Query<Entity, With<DockRoot>>,
    existing_panes: Query<(Entity, &PaneId), With<Pane>>,
) {
    if !layout.is_changed() && !rebuild.0 {
        return;
    }
    rebuild.0 = false;

    if let Err(e) = layout.validate() {
        error!("[tempera-dock] refusing to build an invalid layout: {e}");
        return;
    }

    // Panes that exist now and are still declared keep their entity — and with
    // it every child some other crate parented in.
    let mut kept: Vec<(PaneId, Entity)> = Vec::new();
    for (entity, id) in &existing_panes {
        if layout.root.find_pane(id.as_str()).is_some() {
            kept.push((id.clone(), entity));
        }
    }

    // Detach survivors before the old tree goes, so despawning it does not take
    // their content with it.
    for (_, entity) in &kept {
        commands.entity(*entity).remove::<ChildOf>();
    }
    for root in &roots {
        commands.entity(root).despawn();
    }

    registry.clear();
    let root = commands
        .spawn((
            DockRoot,
            DockNode,
            Name::new("DockRoot"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .id();

    let child = spawn_node(
        &mut commands,
        &mut registry,
        &style,
        &layout.root,
        &kept,
        // The root behaves as a single-child column: whatever the tree's top
        // node is, it fills the window.
        Axis::Column,
    );
    commands.entity(root).add_child(child);
}

/// Spawn one declared node and its subtree, returning the entity.
fn spawn_node(
    commands: &mut Commands,
    registry: &mut PaneRegistry,
    style: &DividerStyle,
    tree: &DockTree,
    kept: &[(PaneId, Entity)],
    parent_axis: Axis,
) -> Entity {
    match tree {
        DockTree::Pane {
            id,
            size,
            min_size,
            visibility,
        } => {
            let entity = kept
                .iter()
                .find(|(kept_id, _)| kept_id == id)
                .map(|(_, entity)| *entity)
                .unwrap_or_else(|| commands.spawn_empty().id());

            let mut entity_commands = commands.entity(entity);
            entity_commands.insert((
                Pane,
                DockNode,
                id.clone(),
                *size,
                *visibility,
                Name::new(format!("Pane({})", id.as_str())),
                pane_node(*size, *visibility, parent_axis),
                BackgroundColor(Color::NONE),
            ));
            match min_size {
                Some(px) => entity_commands.insert(PaneMinSize(*px)),
                // A pane that had a floor and no longer declares one must lose
                // it, or a reconciled tree keeps enforcing a rule it dropped.
                None => entity_commands.remove::<PaneMinSize>(),
            };

            registry.insert(id, entity);
            entity
        }
        DockTree::Split {
            axis,
            size,
            children,
        } => {
            let entity = commands
                .spawn((
                    Split { axis: *axis },
                    DockNode,
                    *size,
                    PaneVisibility::Shown,
                    Name::new("Split"),
                    split_node(*size, *axis, parent_axis),
                ))
                .id();

            let mut wired: Vec<Entity> = Vec::with_capacity(children.len() * 2 - 1);
            for (i, child) in children.iter().enumerate() {
                if i > 0 {
                    wired.push(spawn_divider(commands, style, *axis));
                }
                wired.push(spawn_node(commands, registry, style, child, kept, *axis));
            }
            commands.entity(entity).add_children(&wired);
            entity
        }
    }
}

/// The seam between two children of a split.
fn spawn_divider(commands: &mut Commands, style: &DividerStyle, axis: Axis) -> Entity {
    let (width, height) = match axis {
        Axis::Row => (Val::Px(style.thickness), Val::Percent(100.0)),
        Axis::Column => (Val::Percent(100.0), Val::Px(style.thickness)),
    };
    commands
        .spawn((
            Divider,
            DockNode,
            Name::new("Divider"),
            Node {
                width,
                height,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .observe(on_divider_drag)
        .observe(on_divider_hover)
        .observe(on_divider_out)
        .id()
}

/// A pane frame.
///
/// `Relative` + `clip` is the overlay contract: an application can spawn an
/// absolutely-positioned child of any pane and have it anchored to that pane,
/// which is what floating chrome uses instead of the dock modelling floating
/// panes itself.
fn pane_node(size: PaneSize, visibility: PaneVisibility, parent_axis: Axis) -> Node {
    let mut node = sized_node(size, parent_axis);
    node.position_type = PositionType::Relative;
    node.overflow = Overflow::clip();
    node.display = display_of(visibility);
    node
}

fn split_node(size: PaneSize, axis: Axis, parent_axis: Axis) -> Node {
    let mut node = sized_node(size, parent_axis);
    node.flex_direction = axis.flex_direction();
    node
}

/// Size a node along its parent's axis.
///
/// The cross axis is always 100%: a child of a row fills the row's height, and
/// a child of a column fills its width. Only the main axis is negotiated.
fn sized_node(size: PaneSize, parent_axis: Axis) -> Node {
    let mut node = Node::default();
    match (size, parent_axis) {
        (PaneSize::Flex(weight), Axis::Row) => {
            node.flex_grow = weight;
            // Flex basis of zero, so `flex_grow` alone decides the split and a
            // pane's content cannot widen it by being wide.
            node.width = Val::Px(0.0);
            node.height = Val::Percent(100.0);
        }
        (PaneSize::Flex(weight), Axis::Column) => {
            node.flex_grow = weight;
            node.height = Val::Px(0.0);
            node.width = Val::Percent(100.0);
        }
        (PaneSize::Fixed(px), Axis::Row) => {
            node.flex_grow = 0.0;
            node.flex_shrink = 0.0;
            node.width = Val::Px(px);
            node.height = Val::Percent(100.0);
        }
        (PaneSize::Fixed(px), Axis::Column) => {
            node.flex_grow = 0.0;
            node.flex_shrink = 0.0;
            node.height = Val::Px(px);
            node.width = Val::Percent(100.0);
        }
    }
    node
}

fn display_of(visibility: PaneVisibility) -> Display {
    if visibility.is_shown() {
        Display::Flex
    } else {
        Display::None
    }
}
