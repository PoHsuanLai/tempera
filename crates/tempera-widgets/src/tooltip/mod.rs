//! Tooltip — contextual help on hover.
//!
//! Add a [`Tooltip`] component to any UI node; on hover (after
//! `delay_ms`) tempera spawns a styled popup anchored to the node,
//! with a small arrow pointing back. Despawns on hover loss.
//!
//! Visuals follow shadcn's tooltip: inverted colors
//! (`bg-foreground`, `text-background`), 6px corner radius, 5px arrow.
//! Position auto-flips (Top → Bottom → Right → Left) if the preferred
//! side doesn't fit, matching armas.
//!
//! ## Composition
//!
//! - [`Tooltip`] — config on the target entity
//! - [`TooltipPopup`] — marker on the spawned popup; carries the
//!   target Entity so the sync system can re-anchor each frame
//! - [`TooltipArrow`] — marker on the arrow child
//!
//! ## Usage
//!
//! ```ignore
//! let button = spawn_button(&mut commands, &style, ButtonContent::text("Save"), ButtonVariant::Default);
//! commands.entity(button).insert(Tooltip::new("Save the project (⌘S)").delay(250));
//! ```

use bevy::prelude::*;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{Tooltip, TooltipArrow, TooltipPopup, TooltipPosition, TooltipShortcutFor};

pub struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(systems::on_hover_start);
        app.add_observer(systems::on_hover_end);
        app.add_systems(
            Update,
            (
                // Before the popup is built, so a command-tracking tooltip
                // shows what is bound now rather than at the previous hover.
                systems::resolve_command_shortcuts,
                systems::open_tooltips,
                systems::sync_popup_positions,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use components::TooltipHover;
    use tempera_input::{
        AppCommandExt, CommandId, CommandLabel, CommandRegistry, Keybind, TemperaInputPlugin,
        dyn_cmd, key, on_press,
    };

    /// An app with one command bound to `chord`, and a button whose tooltip
    /// tracks it. The button is **not** hovered yet.
    ///
    /// The command goes in through `spawn_command`, which is what indexes it in
    /// `CommandRegistry` — a bare `world.spawn` of the same components leaves
    /// the lookup empty and every assertion below vacuously "correct".
    fn app_with(chord: Option<tempera_input::Chord>) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(TemperaInputPlugin::new("test-tooltip-shortcut"))
            .add_systems(Update, systems::resolve_command_shortcuts);

        app.spawn_command(dyn_cmd(
            "view.toggle",
            (CommandLabel::new("Toggle"), on_press(|_| {})),
        ));
        let command = app
            .world()
            .resource::<CommandRegistry>()
            .get("view.toggle")
            .expect("just registered");
        if let Some(chord) = chord {
            app.world_mut().entity_mut(command).insert(Keybind(chord));
        }

        let button = app
            .world_mut()
            .spawn((
                Tooltip::new("Toggle"),
                TooltipShortcutFor(CommandId("view.toggle".to_owned())),
            ))
            .id();

        app.update();
        (app, command, button)
    }

    /// Put the pointer on `button` the way the `Pointer<Over>` observer would,
    /// then run the frame that follows.
    fn hover(app: &mut App, button: Entity) {
        app.world_mut()
            .entity_mut(button)
            .insert(TooltipHover { started_at: 0.0 });
        app.update();
    }

    fn unhover(app: &mut App, button: Entity) {
        app.world_mut().entity_mut(button).remove::<TooltipHover>();
        app.update();
    }

    fn glyphs(app: &App, button: Entity) -> Option<Vec<String>> {
        app.world()
            .get::<Tooltip>(button)
            .expect("button exists")
            .shortcut
            .as_ref()
            .map(|chord| chord.render_order().map(|k| k.glyph()).collect())
    }

    #[test]
    fn hovering_shows_what_the_command_is_bound_to() {
        let (mut app, _, button) = app_with(Some(key(KeyCode::KeyB)));
        hover(&mut app, button);

        assert_eq!(glyphs(&app, button), Some(vec!["B".to_owned()]));
    }

    #[test]
    fn nothing_is_resolved_before_the_pointer_arrives() {
        // The whole design: the chord is not cached at spawn, so an un-hovered
        // tooltip holds none however long it has existed.
        let (app, _, button) = app_with(Some(key(KeyCode::KeyB)));

        assert_eq!(
            glyphs(&app, button),
            None,
            "resolving early would be a cache, which is what this avoids"
        );
    }

    #[test]
    fn a_rebind_between_hovers_shows_the_new_chord() {
        // What a spawn-time copy gets wrong. No invalidation runs here — the
        // second hover simply reads what is bound now.
        let (mut app, command, button) = app_with(Some(key(KeyCode::KeyB)));
        hover(&mut app, button);
        assert_eq!(glyphs(&app, button), Some(vec!["B".to_owned()]));

        unhover(&mut app, button);
        app.world_mut()
            .entity_mut(command)
            .insert(Keybind(key(KeyCode::KeyZ)));
        hover(&mut app, button);

        assert_eq!(glyphs(&app, button), Some(vec!["Z".to_owned()]));
    }

    #[test]
    fn an_unbind_between_hovers_clears_the_chord() {
        // Unbinding is a component *removal*, which a `Changed` filter cannot
        // see. Resolving on hover never asks that question — but the write
        // must still happen, because the entity carries the previous chord.
        let (mut app, command, button) = app_with(Some(key(KeyCode::KeyB)));
        hover(&mut app, button);
        assert!(glyphs(&app, button).is_some(), "bound on the first hover");

        unhover(&mut app, button);
        app.world_mut().entity_mut(command).remove::<Keybind>();
        hover(&mut app, button);

        assert_eq!(
            glyphs(&app, button),
            None,
            "a cleared binding must clear the chord"
        );
    }

    #[test]
    fn a_command_with_no_binding_shows_no_chord() {
        let (mut app, _, button) = app_with(None);
        hover(&mut app, button);

        assert_eq!(glyphs(&app, button), None);
    }

    #[test]
    fn an_id_nothing_claims_shows_no_chord() {
        // Commands are registered by whichever crates are present, so a
        // toolbar naming one from an absent crate is ordinary, not broken.
        let mut app = App::new();
        app.add_plugins(TemperaInputPlugin::new("test-tooltip-unknown"))
            .add_systems(Update, systems::resolve_command_shortcuts);
        let button = app
            .world_mut()
            .spawn((
                Tooltip::new("Nothing"),
                TooltipShortcutFor(CommandId("no.such.command".to_owned())),
            ))
            .id();
        app.update();
        hover(&mut app, button);

        assert_eq!(glyphs(&app, button), None);
    }

    #[test]
    fn a_literal_chord_is_left_alone() {
        // "Press Esc to dismiss" has no command behind it and must survive a
        // hover — the query filter on `TooltipShortcutFor` is the guarantee.
        let (mut app, command, _) = app_with(Some(key(KeyCode::KeyB)));
        let literal = app
            .world_mut()
            .spawn(Tooltip::new("Dismiss").shortcut(KeyCode::Escape))
            .id();

        app.world_mut().entity_mut(command).remove::<Keybind>();
        hover(&mut app, literal);

        assert_eq!(
            glyphs(&app, literal),
            Some(vec!["⎋".to_owned()]),
            "an untracked tooltip keeps what its caller wrote"
        );
    }

    // NOTE — the resolve/spawn *ordering* is not covered here.
    //
    // `resolve_command_shortcuts` must run before `open_tooltips`, or the popup
    // renders the previous hover's chord. Nothing below catches a reversal:
    // spawning a popup needs a `PrimaryWindow` and a laid-out `ComputedNode`,
    // so a headless test never reaches that code, and every assertion here
    // reads `Tooltip.shortcut` rather than the popup.
    //
    // A schedule-graph assertion was tried and deleted — it passed with the
    // two systems reversed, which is worse than no test at all. The `.chain()`
    // in the plugin is what holds the property; covering it for real needs a
    // rendering harness this crate does not have.

    #[test]
    fn a_tooltip_spawned_after_its_command_still_resolves() {
        // Registration order is not a contract: a panel built later must not
        // depend on having existed when its command was registered.
        let (mut app, _, _) = app_with(Some(key(KeyCode::KeyB)));

        let late = app
            .world_mut()
            .spawn((
                Tooltip::new("Late"),
                TooltipShortcutFor(CommandId("view.toggle".to_owned())),
            ))
            .id();
        app.update();
        hover(&mut app, late);

        assert_eq!(glyphs(&app, late), Some(vec!["B".to_owned()]));
    }
    #[test]
    fn a_tooltips_caps_are_the_same_shape_as_any_other() {
        // Guards the *call*, not the callee. `spawn_kbd_in`'s own tests check
        // what it produces; nothing there notices if the tooltip stops
        // calling it — which is exactly the shape of the original bug, where
        // a private fork drifted to 5/1 padding and missed the alignment
        // floor while every kbd test stayed green.
        //
        // Re-forking the renderer inside `spawn_popup` passes every other
        // test in the crate. This one is the only thing that fails.
        //
        // Drives `spawn_popup` directly rather than through a hover: the
        // hover path needs a window, a camera and a measured `ComputedNode`
        // before it will place a popup, none of which this is about.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(bevy::text::TextPlugin)
            .add_plugins(crate::kbd::KbdPlugin);
        app.update();

        let chord = crate::kbd::KbdChord::from(KeyCode::Comma);
        let target = app.world_mut().spawn(Node::default()).id();

        type PopupParams<'w, 's> = (
            Commands<'w, 's>,
            Res<'w, crate::theme::ColorPalette>,
            Res<'w, crate::theme::Typography>,
            Res<'w, crate::theme::FontHandle>,
            Res<'w, crate::theme::Tokens>,
        );
        let world = app.world_mut();
        let mut state: bevy::ecs::system::SystemState<PopupParams> =
            bevy::ecs::system::SystemState::new(world);
        let (mut commands, palette, typography, font, tokens) =
            state.get(world).expect("params validate");
        let metrics = super::spawn::TooltipMetrics::from(&tokens);
        super::spawn::spawn_popup(
            &mut commands,
            target,
            &Tooltip::new("Toggle").shortcut(chord.clone()),
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::new(800.0, 600.0),
            &palette,
            &typography,
            &font,
            &metrics,
        );
        let reference = crate::kbd::spawn_kbd_in(
            &mut commands,
            chord,
            crate::kbd::KbdColors::standard(&palette),
            crate::kbd::Repaint::FollowsPalette,
            &font,
            &typography,
        );
        state.apply(world);
        app.update();

        let want = {
            let world = app.world();
            let cap = world.get::<Children>(reference).expect("caps")[0];
            world.get::<Node>(cap).unwrap().clone()
        };

        // Every cap in the world except the reference row's belongs to the
        // popup.
        let popup_caps: Vec<Node> = {
            let reference_caps: Vec<Entity> = app
                .world()
                .get::<Children>(reference)
                .expect("caps")
                .iter()
                .collect();
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<crate::kbd::KbdCap>>();
            let all: Vec<Entity> = q.iter(world).collect();
            all.into_iter()
                .filter(|e| !reference_caps.contains(e))
                .map(|e| world.get::<Node>(e).unwrap().clone())
                .collect()
        };

        assert!(
            !popup_caps.is_empty(),
            "the popup rendered no keycaps at all"
        );
        for got in popup_caps {
            assert_eq!(
                got.padding, want.padding,
                "the tooltip is not using the shared keycap renderer"
            );
            assert_eq!(
                got.min_width, want.min_width,
                "a tooltip cap missed the alignment floor"
            );
            assert_eq!(got.border, want.border);
            assert_eq!(got.border_radius, want.border_radius);
        }
    }
}
