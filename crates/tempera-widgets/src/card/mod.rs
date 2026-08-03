//! Card — a titled panel whose body collapses.
//!
//! ```ignore
//! let parts = spawn_card(&mut commands, &style, parent, "Transport", CardState::Expanded);
//! commands.entity(parts.body).with_children(|b| { /* section content */ });
//! ```
//!
//! # Collapse is a marker, not a bool
//!
//! [`CardExpanded`] is present on an open card and absent on a closed one,
//! matching [`crate::tree_row::TreeRowExpanded`] and Bevy's own `Checked`.
//! A `CardCollapsed(bool)` field would let a caller write the flag without
//! the art following, and would name the state negatively — `false` meaning
//! open has to be read twice.
//!
//! # It collapses, it does not rebuild
//!
//! Collapsing sets the body's `Display` and nothing else, so the body keeps
//! its children, its layout and any widget state inside across a collapse
//! and re-expand. A card that despawned its contents would lose scroll
//! position and every in-progress edit.
//!
//! # What it does not do
//!
//! No accordion behaviour — closing one card never closes another. Which
//! cards may be open at once is a property of the *panel*, not of any card,
//! and a widget that enforced it would need to know about its siblings.
//! A caller wanting an accordion observes [`CardExpanded`] and closes the
//! rest.

use bevy::prelude::*;

mod components;
mod spawn;
mod systems;

pub use components::{Card, CardBody, CardChevron, CardExpanded, CardHeader, CardState};
pub use spawn::{CardParts, CardStyle, CardTokens, spawn_card};

use crate::cursor::CursorPlugin;
use crate::theme::ThemePlugin;

/// Collapse behaviour and chevron art for [`Card`]s.
pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<CursorPlugin>() {
            app.add_plugins(CursorPlugin);
        }
        app.init_resource::<CardTokens>().add_systems(
            Update,
            (systems::apply_card_body, systems::apply_card_chevron),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{ColorPalette, Tokens};

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(ThemePlugin)
            .init_resource::<CardTokens>()
            .init_resource::<Assets<Image>>()
            .add_systems(
                Update,
                (systems::apply_card_body, systems::apply_card_chevron),
            );
        app
    }

    fn spawn(app: &mut App, state: CardState) -> CardParts {
        let parent = app.world_mut().spawn(Node::default()).id();
        let mut sys: bevy::ecs::system::SystemState<(Commands, CardStyle)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let parts = {
            let (mut commands, style) = sys.get(app.world()).expect("theme present");
            spawn_card(&mut commands, &style, parent, "Section", state)
        };
        sys.apply(app.world_mut());
        parts
    }

    fn display(app: &App, body: Entity) -> Display {
        app.world().get::<Node>(body).unwrap().display
    }

    fn chevron_of(app: &App, card: Entity) -> Entity {
        for child in app.world().get::<Children>(card).unwrap().iter() {
            let Some(kids) = app.world().get::<Children>(child) else {
                continue;
            };
            for grand in kids.iter() {
                if app.world().get::<CardChevron>(grand).is_some() {
                    return grand;
                }
            }
        }
        panic!("a card has a chevron");
    }

    #[test]
    fn a_collapsed_card_hides_its_body_and_an_expanded_one_shows_it() {
        let mut app = test_app();
        let open = spawn(&mut app, CardState::Expanded);
        let shut = spawn(&mut app, CardState::Collapsed);
        app.update();

        assert_eq!(display(&app, open.body), Display::Flex);
        assert_eq!(display(&app, shut.body), Display::None);
        assert!(app.world().get::<CardExpanded>(open.card).is_some());
        assert!(app.world().get::<CardExpanded>(shut.card).is_none());
    }

    #[test]
    fn collapsing_a_card_reaches_its_body() {
        // `Changed<T>` does not fire on removal, and collapsing *is* a
        // removal — so a system filtered on `Changed<CardExpanded>` leaves
        // the body visible after the first collapse and the click reads as
        // ignored. dawai's version stored a `CardCollapsed(bool)` partly to
        // dodge this; a marker plus an unfiltered compare-first system is
        // the fix `tree_row` already uses.
        let mut app = test_app();
        let parts = spawn(&mut app, CardState::Expanded);
        app.update();
        assert_eq!(display(&app, parts.body), Display::Flex);

        app.world_mut()
            .entity_mut(parts.card)
            .remove::<CardExpanded>();
        app.update();
        assert_eq!(display(&app, parts.body), Display::None);

        app.world_mut().entity_mut(parts.card).insert(CardExpanded);
        app.update();
        assert_eq!(display(&app, parts.body), Display::Flex);
    }

    #[test]
    fn the_chevron_follows_the_state_in_both_directions() {
        // dawai's card set the chevron art once, from a system that only
        // looked for chevrons *without* an `ImageNode` — so it pointed down
        // forever regardless of whether the card was open.
        let mut app = test_app();
        let (down, right) = {
            let images = app.world_mut().resource_mut::<Assets<Image>>();
            (images.reserve_handle(), images.reserve_handle())
        };
        {
            let mut tokens = app.world_mut().resource_mut::<CardTokens>();
            tokens.chevron_expanded = Some(down.clone());
            tokens.chevron_collapsed = Some(right.clone());
        }

        let parts = spawn(&mut app, CardState::Expanded);
        app.update();
        let chevron = chevron_of(&app, parts.card);
        assert_eq!(app.world().get::<ImageNode>(chevron).unwrap().image, down);

        app.world_mut()
            .entity_mut(parts.card)
            .remove::<CardExpanded>();
        app.update();
        assert_eq!(app.world().get::<ImageNode>(chevron).unwrap().image, right);
    }

    #[test]
    fn the_chevron_slot_is_reserved_before_the_art_arrives() {
        // Handles load asynchronously. Without a reserved box the title
        // shifts sideways the frame the image lands.
        let mut app = test_app();
        let parts = spawn(&mut app, CardState::Expanded);
        app.update();

        let chevron = chevron_of(&app, parts.card);
        assert!(
            app.world().get::<ImageNode>(chevron).is_none(),
            "no art was supplied"
        );
        assert!(matches!(
            app.world().get::<Node>(chevron).unwrap().width,
            Val::Px(v) if v > 0.0
        ));
    }

    #[test]
    fn a_body_keeps_its_contents_across_a_collapse() {
        // Collapsing sets `Display` and nothing else. A card that despawned
        // its body would lose scroll position and every in-progress edit
        // inside it.
        let mut app = test_app();
        let parts = spawn(&mut app, CardState::Expanded);
        let content = app
            .world_mut()
            .spawn((Node::default(), ChildOf(parts.body)))
            .id();
        app.update();

        app.world_mut()
            .entity_mut(parts.card)
            .remove::<CardExpanded>();
        app.update();
        app.world_mut().entity_mut(parts.card).insert(CardExpanded);
        app.update();

        assert!(app.world().get_entity(content).is_ok(), "content survived");
        assert_eq!(
            app.world().get::<ChildOf>(content).unwrap().0,
            parts.body,
            "content stayed in the body"
        );
    }

    #[test]
    fn a_card_reads_its_geometry_from_the_theme() {
        // The values dawai hardcoded — CARD_BG, CARD_RADIUS, HEADER_HEIGHT.
        let mut app = test_app();
        let parts = spawn(&mut app, CardState::Expanded);
        app.update();

        let tokens = *app.world().resource::<Tokens>();
        let palette = app.world().resource::<ColorPalette>().clone();
        assert_eq!(
            app.world().get::<BackgroundColor>(parts.card).unwrap().0,
            palette.card
        );

        let header = app.world().get::<Children>(parts.card).unwrap()[0];
        assert_eq!(
            app.world().get::<Node>(header).unwrap().height,
            Val::Px(tokens.sizing.control_sm.get())
        );
    }

    #[test]
    fn recolouring_the_palette_reaches_a_chevron() {
        let mut app = test_app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .reserve_handle();
        app.world_mut()
            .resource_mut::<CardTokens>()
            .chevron_expanded = Some(handle);
        let parts = spawn(&mut app, CardState::Expanded);
        app.update();

        let recoloured = Color::srgb(0.2, 0.9, 0.4);
        app.world_mut()
            .resource_mut::<ColorPalette>()
            .muted_foreground = recoloured;
        app.update();

        let chevron = chevron_of(&app, parts.card);
        assert_eq!(
            app.world().get::<ImageNode>(chevron).unwrap().color,
            recoloured
        );
    }
}
