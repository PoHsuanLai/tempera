//! Hiding a node, and the divider beside it.
//!
//! One system over every node, replacing the usual shape — one query and one
//! boolean per named slot, which stops scaling the moment a layout has more
//! than a couple of hideable panes.

use bevy::prelude::*;

use crate::node::{Divider, DockNode, PaneVisibility};

/// Apply [`PaneVisibility`] to `Node::display`, for the node and its divider.
///
/// Runs on change only. Hiding a node also hides the divider it would have
/// been separated by, so a hidden pane leaves no floating seam; which divider
/// that is depends on position, so the parent's child list is consulted rather
/// than a stored reference.
pub(crate) fn apply_visibility(
    changed: Query<(Entity, &PaneVisibility, Option<&ChildOf>), Changed<PaneVisibility>>,
    children_of: Query<&Children>,
    dividers: Query<(), With<Divider>>,
    mut nodes: Query<&mut Node, With<DockNode>>,
) {
    for (entity, visibility, parent) in &changed {
        let display = if visibility.is_shown() {
            Display::Flex
        } else {
            Display::None
        };

        if let Ok(mut node) = nodes.get_mut(entity) {
            node.display = display;
        }

        let Some(divider) = parent
            .map(ChildOf::parent)
            .and_then(|p| children_of.get(p).ok())
            .and_then(|siblings| adjacent_divider(siblings, entity, &dividers))
        else {
            continue;
        };
        if let Ok(mut node) = nodes.get_mut(divider) {
            node.display = display;
        }
    }
}

/// The divider that separates `entity` from the rest of the split.
///
/// The one before it, or — for the first child, which has nothing before it —
/// the one after. Returns `None` for a lone child, which has no seam at all.
fn adjacent_divider(
    siblings: &Children,
    entity: Entity,
    dividers: &Query<(), With<Divider>>,
) -> Option<Entity> {
    let index = siblings.iter().position(|c| c == entity)?;
    let before = index
        .checked_sub(1)
        .and_then(|i| siblings.get(i))
        .copied()
        .filter(|e| dividers.contains(*e));
    before.or_else(|| {
        siblings
            .get(index + 1)
            .copied()
            .filter(|e| dividers.contains(*e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Pane, PaneSize};

    /// A row of two panes with a divider between them, mirroring what the
    /// builder produces.
    fn split_with_two_panes(world: &mut World) -> (Entity, Entity, Entity) {
        let a = world
            .spawn((DockNode, Pane, PaneSize::default(), Node::default()))
            .id();
        let d = world.spawn((DockNode, Divider, Node::default())).id();
        let b = world
            .spawn((DockNode, Pane, PaneSize::default(), Node::default()))
            .id();
        world
            .spawn((DockNode, Node::default()))
            .add_children(&[a, d, b]);
        (a, d, b)
    }

    fn run(world: &mut World) {
        world.run_system_once(apply_visibility).unwrap();
    }

    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn hiding_a_pane_hides_its_divider() {
        let mut world = World::new();
        let (a, d, b) = split_with_two_panes(&mut world);
        world.entity_mut(a).insert(PaneVisibility::Hidden);

        run(&mut world);

        assert_eq!(world.get::<Node>(a).unwrap().display, Display::None);
        assert_eq!(
            world.get::<Node>(d).unwrap().display,
            Display::None,
            "a hidden pane must not leave a floating seam"
        );
        assert_eq!(world.get::<Node>(b).unwrap().display, Display::Flex);
    }

    #[test]
    fn hiding_the_last_child_hides_the_preceding_divider() {
        let mut world = World::new();
        let (a, d, b) = split_with_two_panes(&mut world);
        world.entity_mut(b).insert(PaneVisibility::Hidden);

        run(&mut world);

        assert_eq!(world.get::<Node>(b).unwrap().display, Display::None);
        assert_eq!(world.get::<Node>(d).unwrap().display, Display::None);
        assert_eq!(world.get::<Node>(a).unwrap().display, Display::Flex);
    }

    #[test]
    fn size_survives_a_hide_unhide_cycle() {
        // Visibility must never touch PaneSize, or a pane the user dragged wide
        // would snap back to its declared width every time it is toggled.
        let mut world = World::new();
        let (a, _, _) = split_with_two_panes(&mut world);
        world.entity_mut(a).insert(PaneSize::Flex(3.7));

        world.entity_mut(a).insert(PaneVisibility::Hidden);
        run(&mut world);
        world.entity_mut(a).insert(PaneVisibility::Shown);
        run(&mut world);

        assert_eq!(*world.get::<PaneSize>(a).unwrap(), PaneSize::Flex(3.7));
        assert_eq!(world.get::<Node>(a).unwrap().display, Display::Flex);
    }

    #[test]
    fn a_lone_child_has_no_divider_to_hide() {
        let mut world = World::new();
        let a = world
            .spawn((DockNode, Pane, Node::default(), PaneVisibility::Hidden))
            .id();
        world.spawn((DockNode, Node::default())).add_children(&[a]);

        run(&mut world);

        assert_eq!(world.get::<Node>(a).unwrap().display, Display::None);
    }
}
