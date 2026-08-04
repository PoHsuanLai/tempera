//! An open context menu follows a theme change.
//!
//! # The bug this exists for
//!
//! Every colour in a menu was written once at spawn. That looked safe by
//! analogy with the tooltip, which paints from the palette and has no
//! repaint system either: a tooltip despawns on hover-out, so it cannot
//! outlive a swap and always reads a current palette.
//!
//! A menu is *not* the same, and the difference is the whole reason this
//! file exists. Dismissal is driven by **focus** — clicking away, or
//! Escape — and a theme change touches neither. A menu left open while
//! anything else recolours the app keeps every colour it was born with,
//! and in a light theme its labels stay near-white against a light
//! popover.
//!
//! `the_open_menu_survives_a_theme_change` pins the premise itself,
//! because if a menu ever *did* despawn on a palette change the rest of
//! this file would pass vacuously.

use bevy::prelude::*;
use tempera::context_menu::*;
use tempera::theme::{ColorPalette, ThemePlugin};

/// A headless app with a primary window — `open_requested_menus` returns
/// early without one, so a harness that omits it tests nothing.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .add_plugins(bevy::image::ImagePlugin::default())
        .add_plugins(bevy::text::TextPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(bevy::input_focus::InputFocusPlugin)
        .add_plugins(bevy::picking::PickingPlugin)
        .add_plugins(bevy::picking::InteractionPlugin)
        .add_plugins(ThemePlugin)
        .add_plugins(ContextMenuPlugin)
        .insert_resource(ColorPalette::dark());
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    app
}

fn open(app: &mut App, items: Vec<MenuItemSpec>) {
    app.world_mut().write_message(OpenContextMenu(MenuRequest {
        items,
        anchor: Vec2::new(10.0, 10.0),
    }));
    app.update();
    app.update();
}

fn go_light(app: &mut App) {
    app.world_mut().insert_resource(ColorPalette::light());
    app.update();
    app.update();
}

#[test]
fn the_open_menu_survives_a_theme_change() {
    // The premise every other test here rests on. If a palette swap ever
    // dismissed the menu, the repaint would be dead code and the
    // assertions below would hold for the wrong reason.
    let mut app = app();
    open(&mut app, vec![MenuItemSpec::new("a").label("Alpha")]);
    go_light(&mut app);

    let rows = app
        .world_mut()
        .query_filtered::<Entity, With<TemperaMenuItem>>()
        .iter(app.world())
        .count();
    assert_eq!(
        rows, 1,
        "the menu closed on a theme change, so this file is testing nothing"
    );
}

#[test]
fn the_popover_surface_follows_the_theme() {
    let mut app = app();
    open(&mut app, vec![MenuItemSpec::new("a").label("Alpha")]);
    go_light(&mut app);

    let light = ColorPalette::light();
    let (bg, border) = *app
        .world_mut()
        .query_filtered::<(&BackgroundColor, &BorderColor), With<MenuPopoverSurface>>()
        .iter(app.world())
        .next()
        .map(|(b, r)| (b.0, *r))
        .as_ref()
        .expect("the menu has a surface");
    assert_eq!(bg, light.popover, "the popover kept the old theme");
    assert_eq!(
        border,
        BorderColor::all(light.border),
        "the popover border kept the old theme"
    );
}

#[test]
fn an_ordinary_label_follows_the_theme() {
    // `popover_foreground` is the one that *inverts*, which is why this
    // is the assertion that actually caught the bug — a muted label is
    // mid-grey in both palettes and legible either way.
    let mut app = app();
    open(&mut app, vec![MenuItemSpec::new("a").label("Alpha")]);
    go_light(&mut app);

    let color = app
        .world_mut()
        .query_filtered::<&TextColor, With<MenuItemLabel>>()
        .single(app.world())
        .expect("the row has a label")
        .0;
    assert_eq!(
        color,
        ColorPalette::light().popover_foreground,
        "the label went white-on-white"
    );
}

#[test]
fn a_destructive_label_stays_destructive() {
    // The reason `DestructiveRow` had to become a component. `destructive`
    // was a spawn argument and was consumed there, so a repaint had no way
    // to tell this row from an ordinary one — the failure mode is not a
    // stale colour but a *silently downgraded* one: Delete stops being red.
    let mut app = app();
    open(
        &mut app,
        vec![MenuItemSpec::new("del").label("Delete").destructive()],
    );
    go_light(&mut app);

    let color = app
        .world_mut()
        .query_filtered::<&TextColor, With<MenuItemLabel>>()
        .single(app.world())
        .expect("the row has a label")
        .0;
    assert_eq!(
        color,
        ColorPalette::light().destructive,
        "a destructive row was repainted as an ordinary one"
    );
}

#[test]
fn a_disabled_label_stays_muted() {
    let mut app = app();
    open(
        &mut app,
        vec![MenuItemSpec::new("x").label("Unavailable").disabled()],
    );
    go_light(&mut app);

    let color = app
        .world_mut()
        .query_filtered::<&TextColor, With<MenuItemLabel>>()
        .single(app.world())
        .expect("the row has a label")
        .0;
    assert_eq!(
        color,
        ColorPalette::light().muted_foreground,
        "a disabled row stopped reading as disabled"
    );
}

#[test]
fn a_separator_follows_the_theme() {
    let mut app = app();
    open(
        &mut app,
        vec![
            MenuItemSpec::new("a").label("Alpha"),
            MenuItemSpec::new("b").label("Beta").separator_before(),
        ],
    );
    go_light(&mut app);

    let rule = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<MenuSeparator>>()
        .single(app.world())
        .expect("one separator")
        .0;
    // Sourced from `MenuTokens`, which itself follows the palette — so
    // this pins the seam between the two rather than a literal.
    let want = app.world().resource::<tempera::menu_tokens::MenuTokens>();
    assert_eq!(rule, want.separator, "the separator kept the old theme");
}

#[test]
fn a_submenu_arrow_follows_the_theme() {
    let mut app = app();
    open(
        &mut app,
        vec![
            MenuItemSpec::new("more")
                .label("More")
                .children(vec![MenuItemSpec::new("deep").label("Deeper")]),
        ],
    );
    go_light(&mut app);

    let color = app
        .world_mut()
        .query_filtered::<&TextColor, With<MenuItemMutedText>>()
        .iter(app.world())
        .next()
        .expect("the parent row has an arrow")
        .0;
    assert_eq!(
        color,
        ColorPalette::light().muted_foreground,
        "the submenu arrow kept the old theme"
    );
}
