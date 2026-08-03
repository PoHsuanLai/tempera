//! Tree row — one line of an indented list.
//!
//! Bevy ships no tree or disclosure primitive, and neither did tempera:
//! a file browser, a layer list and an outline view all want the same
//! row and each app was drawing its own. This is that row.
//!
//! A row is: an optional disclosure chevron, an optional icon, a label,
//! and an optional trailing suffix — indented by a depth the caller
//! supplies.
//!
//! ```ignore
//! let row = spawn_tree_row(
//!     &mut commands,
//!     &style,
//!     TreeRowSpec::new("main.rs").suffix("rs").depth(2),
//! );
//! commands.entity(row).observe(|on: On<Pointer<Click>>| { /* … */ });
//! ```
//!
//! # It is a row, singular
//!
//! The widget has no notion of a parent, a child, or a group. It does not
//! know what is above or below it, and it does not own expansion state —
//! [`TreeRowExpanded`] is a marker the caller flips, and this module only
//! swaps the art to match.
//!
//! That is deliberate. Which rows are visible, how deep each sits, and
//! what a collapse hides are *hierarchy* questions, and hierarchy belongs
//! to whatever computed the list. Answering them here would mean owning a
//! tree, and then owning its filtering, and then its persistence — none
//! of which is a widget's job.
//!
//! # Behaviour-free apart from hover
//!
//! The row paints its own hover fill, because that is styling. Everything
//! else — click, drag, context menu, keyboard — is the caller's, attached
//! as observers on the returned entity. A row is a file in one app, a
//! layer in another, and a MIDI device in a third; only the caller knows
//! which.

use bevy::prelude::*;

mod components;
mod spawn;
mod systems;

pub use components::{
    ChevronState, TreeRow, TreeRowChevron, TreeRowExpanded, TreeRowHeader, TreeRowLabel,
    TreeRowSuffix,
};
pub use spawn::{TreeRowSpec, TreeRowStyle, TreeRowTokens, spawn_tree_row};

// `TreeRowPlugin` is defined below; named here so the module's exports read
// as one list.

use crate::cursor::CursorPlugin;
use crate::theme::ThemePlugin;

/// Hover painting and chevron art for [`TreeRow`]s.
pub struct TreeRowPlugin;

impl Plugin for TreeRowPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<CursorPlugin>() {
            app.add_plugins(CursorPlugin);
        }
        // Owns the SVG asset loader and the `UiSvg` → `ImageNode` step that
        // makes an icon child visible at all.
        if !app.is_plugin_added::<bevy_resvg::plugin::SvgPlugin>() {
            app.add_plugins(bevy_resvg::plugin::SvgPlugin);
        }
        app.init_resource::<TreeRowTokens>().add_systems(
            Update,
            (
                systems::repaint_rows.run_if(crate::theme::repaint_needed::<TreeRow>),
                systems::repaint_chevrons,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ColorPalette, FontHandle, Spacing, Typography};
    use bevy_resvg::prelude::{SvgFile, UiSvg};

    /// A world with the theme resources a row reads, and nothing else —
    /// no rendering, so these run headless.
    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<ColorPalette>()
            .init_resource::<Typography>()
            .init_resource::<Spacing>()
            .init_resource::<FontHandle>()
            .init_resource::<TreeRowTokens>()
            .init_resource::<Assets<SvgFile>>()
            .add_systems(
                Update,
                (
                    systems::repaint_rows.run_if(crate::theme::repaint_needed::<TreeRow>),
                    systems::repaint_chevrons,
                ),
            );
        app
    }

    fn spawn(app: &mut App, spec: TreeRowSpec) -> Entity {
        let mut state: bevy::ecs::system::SystemState<(Commands, TreeRowStyle)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let entity = {
            let (mut commands, style) = state.get(app.world()).expect("theme resources present");
            spawn_tree_row(&mut commands, &style, spec)
        };
        state.apply(app.world_mut());
        entity
    }

    fn chevron_of(app: &mut App, row: Entity) -> Option<Entity> {
        let children = app.world().get::<Children>(row)?.iter().collect::<Vec<_>>();
        children
            .into_iter()
            .find(|c| app.world().get::<TreeRowChevron>(*c).is_some())
    }

    #[test]
    fn depth_indents_one_step_per_level() {
        let mut app = test_app();
        let step = app.world().resource::<TreeRowTokens>().indent_step;

        let flat = spawn(&mut app, TreeRowSpec::new("a"));
        let deep = spawn(&mut app, TreeRowSpec::new("b").depth(3));

        assert_eq!(
            app.world().get::<Node>(flat).unwrap().padding.left,
            Val::Px(step)
        );
        assert_eq!(
            app.world().get::<Node>(deep).unwrap().padding.left,
            Val::Px(step * 4.0)
        );
    }

    #[test]
    fn a_leaf_spawns_no_chevron() {
        // A row without a disclosure must not reserve the slot, or every
        // flat list pays an indent it never uses.
        let mut app = test_app();
        let row = spawn(&mut app, TreeRowSpec::new("leaf"));
        assert!(chevron_of(&mut app, row).is_none());
    }

    #[test]
    fn a_chevron_row_records_its_initial_state() {
        let mut app = test_app();
        let open = spawn(
            &mut app,
            TreeRowSpec::new("open").chevron(ChevronState::Expanded),
        );
        let shut = spawn(
            &mut app,
            TreeRowSpec::new("shut").chevron(ChevronState::Collapsed),
        );

        assert!(app.world().get::<TreeRowExpanded>(open).is_some());
        assert!(app.world().get::<TreeRowExpanded>(shut).is_none());
        assert!(chevron_of(&mut app, open).is_some());
    }

    #[test]
    fn the_chevron_slot_is_reserved_before_the_art_arrives() {
        // Handles load asynchronously. Without a reserved slot the label
        // starts flush and jumps right once the image lands.
        let mut app = test_app();
        let row = spawn(
            &mut app,
            TreeRowSpec::new("group").chevron(ChevronState::Collapsed),
        );

        let chevron = chevron_of(&mut app, row).expect("slot exists");
        assert!(
            app.world().get::<UiSvg>(chevron).is_none(),
            "no art was supplied"
        );
        let size = app.world().resource::<TreeRowTokens>().icon_size;
        assert_eq!(
            app.world().get::<Node>(chevron).unwrap().width,
            Val::Px(size)
        );
    }

    #[test]
    fn toggling_expanded_swaps_the_chevron_art() {
        let mut app = test_app();
        let (collapsed, expanded) = {
            let images = app.world_mut().resource_mut::<Assets<SvgFile>>();
            (images.reserve_handle(), images.reserve_handle())
        };
        {
            let mut tokens = app.world_mut().resource_mut::<TreeRowTokens>();
            tokens.chevron_collapsed = Some(collapsed.clone());
            tokens.chevron_expanded = Some(expanded.clone());
        }

        let row = spawn(
            &mut app,
            TreeRowSpec::new("group").chevron(ChevronState::Collapsed),
        );
        app.update();
        let chevron = chevron_of(&mut app, row).unwrap();
        assert_eq!(app.world().get::<UiSvg>(chevron).unwrap().0, collapsed);

        app.world_mut().entity_mut(row).insert(TreeRowExpanded);
        app.update();
        assert_eq!(app.world().get::<UiSvg>(chevron).unwrap().0, expanded);

        // Removing the marker must repaint too — `Changed` does not fire
        // for a removal, so a naive system leaves it pointing open.
        app.world_mut().entity_mut(row).remove::<TreeRowExpanded>();
        app.update();
        assert_eq!(app.world().get::<UiSvg>(chevron).unwrap().0, collapsed);
    }

    #[test]
    fn a_suffix_is_a_separate_child() {
        let mut app = test_app();
        let with = spawn(&mut app, TreeRowSpec::new("kick").suffix("wav"));
        let without = spawn(&mut app, TreeRowSpec::new("kick"));

        let count = |e: Entity, app: &App| {
            app.world()
                .get::<Children>(e)
                .map(|c| {
                    c.iter()
                        .filter(|ch| app.world().get::<TreeRowSuffix>(*ch).is_some())
                        .count()
                })
                .unwrap_or(0)
        };
        assert_eq!(count(with, &app), 1);
        assert_eq!(count(without, &app), 0);
    }

    #[test]
    fn a_long_label_is_truncated_at_spawn() {
        let mut app = test_app();
        let cap = app
            .world()
            .resource::<TreeRowTokens>()
            .label_max_chars
            .unwrap();
        let row = spawn(&mut app, TreeRowSpec::new("x".repeat(cap + 20)));

        let label = app
            .world()
            .get::<Children>(row)
            .unwrap()
            .iter()
            .find(|c| app.world().get::<TreeRowLabel>(*c).is_some())
            .unwrap();
        let text = app.world().get::<Text>(label).unwrap();
        assert_eq!(text.0.chars().count(), cap);
        assert!(text.0.ends_with('…'));
    }

    #[test]
    fn a_header_reads_brighter_than_a_leaf() {
        let mut app = test_app();
        let header = spawn(&mut app, TreeRowSpec::new("Group").header());
        let leaf = spawn(&mut app, TreeRowSpec::new("item"));
        app.update();

        let palette = app.world().resource::<ColorPalette>().clone();
        let color_of = |row: Entity, app: &App| {
            let label = app
                .world()
                .get::<Children>(row)
                .unwrap()
                .iter()
                .find(|c| app.world().get::<TreeRowLabel>(*c).is_some())
                .unwrap();
            app.world().get::<TextColor>(label).unwrap().0
        };
        assert_eq!(color_of(header, &app), palette.foreground);
        assert_eq!(color_of(leaf, &app), palette.muted_foreground);
    }

    #[test]
    fn recolouring_the_palette_reaches_a_rows_label() {
        // A row paints two things: its own background, and its label's
        // colour one level down. The label is the half a per-entity filter
        // misses most easily, because nothing about the *label* changed —
        // only the palette did.
        let mut app = test_app();
        let leaf = spawn(&mut app, TreeRowSpec::new("item"));
        app.update();

        let recoloured = Color::srgb(0.1, 0.8, 0.3);
        app.world_mut()
            .resource_mut::<ColorPalette>()
            .muted_foreground = recoloured;
        app.update();

        let label = app
            .world()
            .get::<Children>(leaf)
            .unwrap()
            .iter()
            .find(|c| app.world().get::<TreeRowLabel>(*c).is_some())
            .expect("a leaf has a label");
        assert_eq!(
            app.world().get::<TextColor>(label).unwrap().0,
            recoloured,
            "a palette swap must reach a label nobody touched"
        );
    }

    #[test]
    fn children_do_not_steal_the_rows_clicks() {
        // Every part of a row must resolve to the row, or a click lands on
        // whichever child happened to be under the cursor.
        let mut app = test_app();
        let row = spawn(
            &mut app,
            TreeRowSpec::new("kick")
                .suffix("wav")
                .chevron(ChevronState::Collapsed),
        );

        let children: Vec<Entity> = app.world().get::<Children>(row).unwrap().iter().collect();
        assert!(!children.is_empty());
        for child in children {
            assert_eq!(
                app.world()
                    .get::<bevy::picking::Pickable>(child)
                    .map(|p| p.is_hoverable),
                Some(false),
                "a row child must not absorb pointer events"
            );
        }
    }
}
