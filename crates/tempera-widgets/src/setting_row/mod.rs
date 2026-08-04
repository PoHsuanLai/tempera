//! Setting row — a labelled control in a settings form.
//!
//! A row is: a label, an optional description under it, and a fixed-width
//! **control slot** the caller fills with one widget. Plus
//! [`spawn_section_header`] for the headings that group them.
//!
//! ```ignore
//! spawn_section_header(&mut commands, &style, body, "EXPORT DEFAULTS");
//!
//! let slot = spawn_setting_row(
//!     &mut commands,
//!     &style,
//!     body,
//!     SettingRowSpec::new("Auto Save").description("Save every 5 minutes"),
//! );
//! let sw = spawn_switch(&mut commands, &switch_style, enabled);
//! commands.entity(sw).insert(ChildOf(slot)).observe(
//!     |on: On<ValueChange<bool>>, mut prefs: ResMut<MyPrefs>| {
//!         prefs.auto_save = on.value;
//!     },
//! );
//! ```
//!
//! # Why this is not [`list_row`](crate::list_row)
//!
//! The two look alike — a bold line over a muted line, with something on
//! the right — and the temptation is to ship one widget with a flag. The
//! differences are behavioural, not cosmetic, and each flag would be a
//! parameter plus a conditional:
//!
//! | | `setting_row` | [`list_row`](crate::list_row) |
//! | --- | --- | --- |
//! | identity | none — the fields are named in source | `ListRowId`, to survive a reconcile |
//! | hover | none — a row is not selectable | tinted |
//! | control slot | **fixed width**, one widget | **`min_width`**, N widgets |
//!
//! The slot width is the clearest of the three. A settings form reads as a
//! column of aligned controls, so the slot is fixed and a wider control is
//! clipped rather than allowed to ragged the edge. A list row's trailing
//! content is a trash button beside a switch, or a keycap beside a reset
//! link — unpredictable, so it gets a floor and grows.
//!
//! A form row edits a **known field**; a list row displays a **discovered
//! record**. Different jobs, and neither is a mode of the other.
//!
//! # No systems, no state
//!
//! [`SettingRowPlugin`] registers [`SettingRowTokens`] and nothing else.
//! The row paints once at spawn and never repaints, because it has no
//! interactive state of its own — the control it holds owns all of that.

use bevy::prelude::*;

mod components;
mod spawn;
mod systems;

pub use components::{
    SettingRow, SettingRowControl, SettingRowDescription, SettingRowLabel, SettingSection,
    SettingSectionLabel,
};
pub use spawn::{
    SettingRowSpec, SettingRowStyle, SettingRowTokens, spawn_section_header, spawn_setting_row,
};

use crate::theme::ThemePlugin;

/// Registers [`SettingRowTokens`] and one repaint system.
pub struct SettingRowPlugin;

impl Plugin for SettingRowPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.init_resource::<SettingRowTokens>().add_systems(
            Update,
            // Only on the frames the theme moved: a row has no per-entity
            // trigger of its own, so staleness lives in the run condition.
            systems::repaint_text.run_if(crate::theme::palette_changed),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ColorPalette, FontHandle, Spacing, Typography};

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<ColorPalette>()
            .init_resource::<Typography>()
            .init_resource::<Spacing>()
            .init_resource::<FontHandle>()
            .init_resource::<SettingRowTokens>();
        app
    }

    /// Spawn a row into a fresh parent; returns `(parent, control_slot)`.
    fn spawn(app: &mut App, spec: SettingRowSpec) -> (Entity, Entity) {
        let parent = app.world_mut().spawn(Node::default()).id();
        let mut state: bevy::ecs::system::SystemState<(Commands, SettingRowStyle)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let slot = {
            let (mut commands, style) = state.get(app.world()).expect("theme resources present");
            spawn_setting_row(&mut commands, &style, parent, spec)
        };
        state.apply(app.world_mut());
        (parent, slot)
    }

    fn descendants(app: &App, root: Entity) -> Vec<Entity> {
        let mut out = Vec::new();
        fn walk(app: &App, e: Entity, out: &mut Vec<Entity>) {
            if let Some(kids) = app.world().get::<Children>(e) {
                for k in kids.iter() {
                    out.push(k);
                    walk(app, k, out);
                }
            }
        }
        walk(app, root, &mut out);
        out
    }

    #[test]
    fn the_row_parents_itself_and_returns_the_slot() {
        // The slot is the only seam, so it is the only return. The
        // implementation this replaces returned `(row, slot)` and every one
        // of its 21 call sites discarded the row.
        let mut app = test_app();
        let (parent, slot) = spawn(&mut app, SettingRowSpec::new("Auto Save"));

        assert!(app.world().get::<SettingRowControl>(slot).is_some());
        let row = app.world().get::<ChildOf>(slot).unwrap().parent();
        assert_eq!(app.world().get::<ChildOf>(row).unwrap().parent(), parent);
    }

    #[test]
    fn the_control_slot_is_fixed_width_not_a_floor() {
        // The opposite choice from `list_row`, and the reason these are two
        // widgets: a form reads as a column of aligned controls.
        let mut app = test_app();
        let (_, slot) = spawn(&mut app, SettingRowSpec::new("Format"));

        let width = app.world().resource::<SettingRowTokens>().control_width;
        let node = app.world().get::<Node>(slot).unwrap();
        assert_eq!(node.width, Val::Px(width));
        assert_eq!(
            node.min_width,
            Val::Auto,
            "a floor would let one long control ragged the column"
        );
    }

    #[test]
    fn one_token_governs_every_control_width() {
        // The bug this fixes: a CONTROL_WIDTH constant existed and the
        // literal 200.0 was still hand-written at 8 sites to match it.
        let mut app = test_app();
        {
            let mut tokens = app.world_mut().resource_mut::<SettingRowTokens>();
            tokens.control_width = 320.0;
        }
        let (_, slot) = spawn(&mut app, SettingRowSpec::new("Format"));

        assert_eq!(
            app.world().get::<Node>(slot).unwrap().width,
            Val::Px(320.0),
            "restyling the form must not require touching call sites"
        );
    }

    #[test]
    fn a_description_is_optional() {
        let mut app = test_app();
        let (bare, _) = spawn(&mut app, SettingRowSpec::new("Language"));
        let (full, _) = spawn(
            &mut app,
            SettingRowSpec::new("Language").description("Interface language"),
        );

        let count = |app: &App, root: Entity| {
            descendants(app, root)
                .into_iter()
                .filter(|e| app.world().get::<SettingRowDescription>(*e).is_some())
                .count()
        };
        assert_eq!(count(&app, bare), 0);
        assert_eq!(count(&app, full), 1);
    }

    #[test]
    fn the_label_reads_brighter_than_the_description() {
        let mut app = test_app();
        let (parent, _) = spawn(
            &mut app,
            SettingRowSpec::new("Auto Save").description("Save every 5 minutes"),
        );

        let palette = app.world().resource::<ColorPalette>().clone();
        let find = |app: &App, pred: &dyn Fn(&App, Entity) -> bool| {
            descendants(app, parent)
                .into_iter()
                .find(|e| pred(app, *e))
                .map(|e| app.world().get::<TextColor>(e).unwrap().0)
        };
        let label = find(&app, &|a, e| a.world().get::<SettingRowLabel>(e).is_some());
        let desc = find(&app, &|a, e| {
            a.world().get::<SettingRowDescription>(e).is_some()
        });

        assert_eq!(label, Some(palette.foreground));
        assert_eq!(desc, Some(palette.muted_foreground));
    }

    #[test]
    fn a_row_has_no_interaction_of_its_own() {
        // A form row is not selectable — the control it holds owns every
        // bit of interactive state. `list_row` is the one that hovers.
        let mut app = test_app();
        let (parent, _) = spawn(&mut app, SettingRowSpec::new("Auto Save"));
        let row = app
            .world()
            .get::<Children>(parent)
            .unwrap()
            .iter()
            .next()
            .unwrap();

        assert!(
            app.world().get::<Interaction>(row).is_none(),
            "a form row must not be a pointer target"
        );
        // `Node` requires `BackgroundColor`, so one always exists; what
        // matters is that this widget never paints it.
        assert_eq!(
            app.world().get::<BackgroundColor>(row).map(|c| c.0),
            Some(Color::NONE),
            "a form row has no fill of its own"
        );
    }

    #[test]
    fn a_section_header_carries_its_text() {
        let mut app = test_app();
        let parent = app.world_mut().spawn(Node::default()).id();
        let mut state: bevy::ecs::system::SystemState<(Commands, SettingRowStyle)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        {
            let (mut commands, style) = state.get(app.world()).expect("theme resources present");
            spawn_section_header(&mut commands, &style, parent, "EXPORT DEFAULTS");
        }
        state.apply(app.world_mut());

        let text = descendants(&app, parent)
            .into_iter()
            .find_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()));
        assert_eq!(text.as_deref(), Some("EXPORT DEFAULTS"));
    }
}
