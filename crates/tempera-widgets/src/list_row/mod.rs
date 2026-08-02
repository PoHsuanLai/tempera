//! List row — one addressable line of a reconciled list.
//!
//! A row is: an identity, a leading column (title, optional meta and badge,
//! optional subtitle), and a **trailing slot the caller fills with any
//! number of widgets**.
//!
//! ```ignore
//! let parts = spawn_list_row(
//!     &mut commands,
//!     &style,
//!     ListRowSpec::new("com.acme.reverb", "Reverb")
//!         .meta("v1.2.0")
//!         .badge("Audio Effect")
//!         .subtitle("A plate reverb with modulation"),
//! );
//! commands.entity(parts.row).insert(ChildOf(section));
//! // Two controls in one row — the thing a form row cannot do.
//! let del = spawn_button(&mut commands, &button_style, /* … */);
//! let sw = spawn_switch(&mut commands, &switch_style, true);
//! commands.entity(del).insert(ChildOf(parts.trail));
//! commands.entity(sw).insert(ChildOf(parts.trail));
//! ```
//!
//! # Why this is not [`tree_row`](crate::tree_row), and not a form row
//!
//! Three row widgets sound like two too many, so the boundaries are worth
//! stating.
//!
//! [`tree_row`](crate::tree_row) is about **depth**: a chevron and an
//! indent, one line tall, for hierarchy. This one has neither and is two
//! lines tall.
//!
//! A **form row** — label on the left, one control on the right — is about
//! *editing a known field*. Its slot is fixed-width and holds one widget,
//! and it needs no identity because the set of fields is written in the
//! source.
//!
//! A **list row** is about *displaying a discovered record*. It is
//! reconciled — filtered, sorted, re-emitted when its source changes — so
//! it needs [`ListRowId`] to be addressable across a respawn, and its
//! trailing content is not predictable, so the slot holds *N* widgets and
//! is sized by `min_width`.
//!
//! That difference is not theoretical. In the code this widget was
//! extracted from, two features needed exactly this and the form row could
//! express neither, so each hand-rolled its own — 220 lines and 162 lines,
//! sharing about 65 lines verbatim, including the same three
//! `&description[..60]` byte-slices that panic on any non-ASCII input.
//!
//! # Behaviour-free apart from hover
//!
//! The row paints its own hover fill, because that is styling. Everything
//! else — click, drag, context menu, what the trailing widgets *do* — is
//! the caller's, attached to [`ListRowParts::row`]. A row is an extension
//! in one app, a keybinding in another, a layer in a third; only the caller
//! knows which.

use bevy::prelude::*;

mod components;
mod spawn;
mod systems;

pub use components::{
    ListRow, ListRowBadge, ListRowId, ListRowLead, ListRowMeta, ListRowSubtitle, ListRowTitle,
    ListRowTrail,
};
pub use spawn::{ListRowParts, ListRowSpec, ListRowStyle, ListRowTokens, spawn_list_row};

use crate::cursor::CursorPlugin;
use crate::theme::ThemePlugin;

/// Hover painting for [`ListRow`]s.
pub struct ListRowPlugin;

impl Plugin for ListRowPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<CursorPlugin>() {
            app.add_plugins(CursorPlugin);
        }
        app.init_resource::<ListRowTokens>()
            .add_systems(Update, systems::repaint_rows);
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
            .init_resource::<ListRowTokens>()
            .add_systems(Update, systems::repaint_rows);
        app
    }

    fn spawn(app: &mut App, spec: ListRowSpec) -> ListRowParts {
        let mut state: bevy::ecs::system::SystemState<(Commands, ListRowStyle)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let parts = {
            let (mut commands, style) = state.get(app.world()).expect("theme resources present");
            spawn_list_row(&mut commands, &style, spec)
        };
        state.apply(app.world_mut());
        parts
    }

    fn children_of(app: &App, entity: Entity) -> Vec<Entity> {
        app.world()
            .get::<Children>(entity)
            .map(|c| c.iter().collect())
            .unwrap_or_default()
    }

    #[test]
    fn a_row_carries_its_id() {
        // The whole reason this is not a form row: a reconcile addresses a
        // row by id, because a respawn invalidates its entity.
        let mut app = test_app();
        let parts = spawn(&mut app, ListRowSpec::new("com.acme.reverb", "Reverb"));

        assert_eq!(
            app.world().get::<ListRowId>(parts.row).map(|i| i.0.clone()),
            Some("com.acme.reverb".to_string())
        );
    }

    #[test]
    fn the_trailing_slot_holds_more_than_one_widget() {
        // A form row's slot is fixed-width and holds one control. This is
        // the case that forced two hand-rolled copies.
        let mut app = test_app();
        let parts = spawn(&mut app, ListRowSpec::new("id", "Title"));

        let a = app.world_mut().spawn(ChildOf(parts.trail)).id();
        let b = app.world_mut().spawn(ChildOf(parts.trail)).id();

        let kids = children_of(&app, parts.trail);
        assert!(kids.contains(&a) && kids.contains(&b));
        assert_eq!(kids.len(), 2);
    }

    #[test]
    fn the_trailing_slot_has_a_floor_not_a_fixed_width() {
        // Fixed width truncates a two-widget trail; a floor lets it grow.
        let mut app = test_app();
        let parts = spawn(&mut app, ListRowSpec::new("id", "Title"));

        let node = app.world().get::<Node>(parts.trail).unwrap();
        let floor = app.world().resource::<ListRowTokens>().trail_min_width;
        assert_eq!(node.min_width, Val::Px(floor));
        assert_eq!(
            node.width,
            Val::Auto,
            "a fixed width would clip the content"
        );
    }

    #[test]
    fn optional_parts_are_absent_rather_than_empty() {
        let mut app = test_app();
        let bare = spawn(&mut app, ListRowSpec::new("a", "Title"));
        let full = spawn(
            &mut app,
            ListRowSpec::new("b", "Title")
                .meta("v1.0")
                .badge("Instrument")
                .subtitle("Does a thing"),
        );

        let count = |app: &App, root: Entity, pred: &dyn Fn(&App, Entity) -> bool| {
            fn walk(app: &App, e: Entity, out: &mut Vec<Entity>) {
                if let Some(kids) = app.world().get::<Children>(e) {
                    for k in kids.iter() {
                        out.push(k);
                        walk(app, k, out);
                    }
                }
            }
            let mut all = Vec::new();
            walk(app, root, &mut all);
            all.into_iter().filter(|e| pred(app, *e)).count()
        };

        let has_meta = |app: &App, e: Entity| app.world().get::<ListRowMeta>(e).is_some();
        let has_badge = |app: &App, e: Entity| app.world().get::<ListRowBadge>(e).is_some();
        let has_sub = |app: &App, e: Entity| app.world().get::<ListRowSubtitle>(e).is_some();

        assert_eq!(count(&app, bare.row, &has_meta), 0);
        assert_eq!(count(&app, bare.row, &has_badge), 0);
        assert_eq!(count(&app, bare.row, &has_sub), 0);
        assert_eq!(count(&app, full.row, &has_meta), 1);
        assert_eq!(count(&app, full.row, &has_badge), 1);
        assert_eq!(count(&app, full.row, &has_sub), 1);
    }

    #[test]
    fn a_long_subtitle_is_truncated_by_chars_not_bytes() {
        // Both implementations this replaces did `&description[..60]`,
        // which panics the moment a multi-byte codepoint straddles 60.
        let mut app = test_app();
        {
            let mut tokens = app.world_mut().resource_mut::<ListRowTokens>();
            tokens.subtitle_max_chars = Some(5);
        }
        let parts = spawn(
            &mut app,
            ListRowSpec::new("id", "Title").subtitle("日本語のファイル名です"),
        );

        let subtitle = children_of(&app, parts.lead)
            .into_iter()
            .find(|e| app.world().get::<ListRowSubtitle>(*e).is_some())
            .expect("subtitle exists");
        let text = app.world().get::<Text>(subtitle).unwrap();
        assert_eq!(text.0.chars().count(), 5);
        assert!(text.0.ends_with('…'));
    }

    #[test]
    fn hover_tints_the_row_and_leaving_clears_it() {
        let mut app = test_app();
        let parts = spawn(&mut app, ListRowSpec::new("id", "Title"));
        app.update();

        let palette = app.world().resource::<ColorPalette>().clone();
        assert_eq!(
            app.world().get::<BackgroundColor>(parts.row).unwrap().0,
            Color::NONE
        );

        *app.world_mut().get_mut::<Interaction>(parts.row).unwrap() = Interaction::Hovered;
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(parts.row).unwrap().0,
            palette.muted
        );

        *app.world_mut().get_mut::<Interaction>(parts.row).unwrap() = Interaction::None;
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(parts.row).unwrap().0,
            Color::NONE
        );
    }

    #[test]
    fn lead_children_do_not_steal_the_rows_clicks() {
        // Every part of the leading column must resolve to the row, or a
        // click lands on whichever text happened to be under the cursor.
        // The trailing slot is deliberately NOT ignored — its widgets are
        // interactive.
        let mut app = test_app();
        let parts = spawn(
            &mut app,
            ListRowSpec::new("id", "Title").subtitle("Description"),
        );

        for child in children_of(&app, parts.lead) {
            assert_eq!(
                app.world()
                    .get::<bevy::picking::Pickable>(child)
                    .map(|p| p.is_hoverable),
                Some(false),
                "a leading-column child must not absorb pointer events"
            );
        }
        assert!(
            app.world()
                .get::<bevy::picking::Pickable>(parts.trail)
                .is_none(),
            "the trailing slot must stay pickable — it holds real controls"
        );
    }
}
