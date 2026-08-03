//! The plugin.

use bevy::prelude::*;

use crate::build::{DockBuildSet, RebuildRequested, build_dock};
use crate::page::apply_active_page;
use crate::registry::PaneRegistry;
use crate::resize::DividerStyle;
use crate::tree::{DockLayout, DockTree};
use crate::visibility::apply_visibility;

/// Panes, splits, dividers and their persistence.
///
/// Insert a [`DockLayout`] to say what the tree should be; without one the dock
/// comes up as a single unnamed pane, which is a working shell with nowhere
/// interesting to put anything.
///
/// ```
/// use bevy::prelude::*;
/// use tempera_dock::{Axis, DockLayout, DockTree, TemperaDockPlugin};
///
/// let mut app = App::new();
/// app.add_plugins(TemperaDockPlugin)
///     .insert_resource(DockLayout::new(DockTree::split(
///         Axis::Row,
///         [DockTree::pane("sidebar"), DockTree::pane("main").flex(4.0)],
///     )));
/// ```
pub struct TemperaDockPlugin;

impl Plugin for TemperaDockPlugin {
    fn build(&self, app: &mut App) {
        // No page state is initialized here: `ActivePage` lives on whichever
        // pane holds pages, so an app with none pays nothing.
        app.init_resource::<PaneRegistry>()
            .init_resource::<RebuildRequested>()
            .init_resource::<DividerStyle>()
            .init_resource::<crate::window_chrome::WindowChromeInset>();

        // A host that inserts its own layout wins; this is only so the dock is
        // never in a state with no tree at all.
        if !app.world().contains_resource::<DockLayout>() {
            app.insert_resource(DockLayout::new(DockTree::pane("main")));
        }

        // The build runs in `Update` rather than `Startup` because it is the
        // same code path that reconciles later changes — a separate startup
        // path would be a second implementation to keep in step.
        app.add_systems(Update, build_dock.in_set(DockBuildSet))
            .add_systems(
                Update,
                (apply_visibility, apply_active_page).after(DockBuildSet),
            )
            // Before the build: a host positioning its first title-bar
            // control reads the inset during layout, so it has to be current
            // by then.
            .add_systems(
                Update,
                crate::window_chrome::sync_window_inset.before(DockBuildSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Axis, Divider, Pane, PaneId};

    fn app_with(layout: DockLayout) -> App {
        let mut app = App::new();
        app.add_plugins(TemperaDockPlugin).insert_resource(layout);
        app
    }

    #[test]
    fn the_plugin_builds_a_declared_tree() {
        let mut app = app_with(DockLayout::new(DockTree::split(
            Axis::Row,
            [DockTree::pane("a"), DockTree::pane("b")],
        )));
        app.update();

        let registry = app.world().resource::<PaneRegistry>();
        assert_eq!(registry.len(), 2);
        assert!(registry.get("a").is_some());
        assert!(registry.get("b").is_some());
    }

    #[test]
    fn a_dock_with_no_declared_layout_still_comes_up() {
        let mut app = App::new();
        app.add_plugins(TemperaDockPlugin);
        app.update();

        assert_eq!(app.world().resource::<PaneRegistry>().len(), 1);
    }

    #[test]
    fn n_children_produce_n_minus_one_dividers() {
        let mut app = app_with(DockLayout::new(DockTree::split(
            Axis::Row,
            [
                DockTree::pane("a"),
                DockTree::pane("b"),
                DockTree::pane("c"),
            ],
        )));
        app.update();

        let dividers = app
            .world_mut()
            .query_filtered::<(), With<Divider>>()
            .iter(app.world())
            .count();
        assert_eq!(dividers, 2);
    }

    #[test]
    fn an_invalid_layout_is_refused_rather_than_half_built() {
        let mut app = app_with(DockLayout::new(DockTree::split(
            Axis::Row,
            [DockTree::pane("dup"), DockTree::pane("dup")],
        )));
        app.update();

        assert!(
            app.world().resource::<PaneRegistry>().is_empty(),
            "a duplicate id must build nothing, not build one of the two"
        );
    }

    #[test]
    fn content_parented_into_a_pane_survives_a_layout_change() {
        // The property the whole reconcile exists for: a resize or a sibling
        // appearing must not make every panel in the app re-spawn.
        #[derive(Component)]
        struct MyPanel;

        let mut app = app_with(DockLayout::new(DockTree::split(
            Axis::Row,
            [DockTree::pane("a"), DockTree::pane("b")],
        )));
        app.update();

        let pane = app.world().resource::<PaneRegistry>().get("a").unwrap();
        let content = app.world_mut().spawn((MyPanel, ChildOf(pane))).id();

        // Same panes, different weights — a reconcile, not a rebuild.
        app.world_mut()
            .insert_resource(DockLayout::new(DockTree::split(
                Axis::Row,
                [DockTree::pane("a").flex(3.0), DockTree::pane("b")],
            )));
        app.update();

        assert!(
            app.world().get_entity(content).is_ok(),
            "content must outlive a layout change"
        );
        assert_eq!(
            app.world().resource::<PaneRegistry>().get("a"),
            Some(pane),
            "a surviving pane keeps its entity"
        );
        assert_eq!(
            app.world().get::<ChildOf>(content).map(ChildOf::parent),
            Some(pane),
            "and content stays parented to it"
        );
    }

    #[test]
    fn a_pane_dropped_from_the_layout_takes_its_content_with_it() {
        #[derive(Component)]
        struct MyPanel;

        let mut app = app_with(DockLayout::new(DockTree::split(
            Axis::Row,
            [DockTree::pane("a"), DockTree::pane("doomed")],
        )));
        app.update();

        let pane = app
            .world()
            .resource::<PaneRegistry>()
            .get("doomed")
            .unwrap();
        let content = app.world_mut().spawn((MyPanel, ChildOf(pane))).id();

        app.world_mut()
            .insert_resource(DockLayout::new(DockTree::pane("a")));
        app.update();

        assert!(app.world().get_entity(content).is_err());
        assert!(
            app.world()
                .resource::<PaneRegistry>()
                .get("doomed")
                .is_none()
        );
    }

    #[test]
    fn every_pane_carries_its_id() {
        let mut app = app_with(DockLayout::new(DockTree::split(
            Axis::Row,
            [DockTree::pane("a"), DockTree::pane("b")],
        )));
        app.update();

        let mut ids: Vec<String> = app
            .world_mut()
            .query_filtered::<&PaneId, With<Pane>>()
            .iter(app.world())
            .map(|id| id.0.clone())
            .collect();
        ids.sort();
        assert_eq!(ids, ["a", "b"]);
    }
}
