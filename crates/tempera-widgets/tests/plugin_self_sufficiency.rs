//! A plugin must bring what its own systems read.
//!
//! Every system tempera schedules is validated against the world before it
//! runs, and a missing `Res<T>` is a hard failure rather than an empty read. So
//! a plugin that adds a system reading `T` without inserting `T` does not
//! degrade — it panics on the consumer's first frame, in a system whose name is
//! hidden unless they happen to build with `bevy/debug`.
//!
//! Two such gaps shipped, and they are the reason this file exists:
//!
//! - `MenuTokens` had a `Default` and no `init_resource` call anywhere in the
//!   crate, while `paint_item_highlight` and `MenuStyle` both read it.
//! - `ContextMenuPlugin` added `InputDispatchPlugin` — which schedules
//!   `dispatch_focused_input` — but not `InputFocusPlugin`, which is what
//!   actually inserts the `InputFocus` those systems read.
//!
//! Both were invisible to the existing tests because those build worlds by
//! hand rather than by adding the plugin and running a frame. These add the
//! plugin the way a consumer does, then run frames.

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use tempera::menu_tokens::MenuTokens;

/// The minimum an app needs before tempera's own plugins are the variable
/// under test.
///
/// Deliberately *not* `DefaultPlugins`: that would supply `InputFocus` and
/// friends itself and mask exactly the gaps being tested.
///
/// `ImagePlugin` is here because it registers `Assets<Image>`, which the SVG
/// rasteriser writes into. That one is *not* a tempera gap: `SvgPlugin` is
/// tempera's to add and does add itself, but the `Image` asset type belongs to
/// bevy's rendering stack, and a widget library that pulled that in would make
/// every headless consumer pay for a renderer. Supplying it is the app's job,
/// which is the line this file is drawing — a plugin owes its own resources,
/// not the engine's.
fn bare_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .add_plugins(bevy::image::ImagePlugin::default())
        .add_plugins(bevy::input::InputPlugin);
    app
}

#[test]
fn the_context_menu_plugin_brings_its_own_menu_tokens() {
    let mut app = bare_app();
    app.add_plugins(tempera::context_menu::ContextMenuPlugin);

    assert!(
        app.world().get_resource::<MenuTokens>().is_some(),
        "systems reading MenuTokens were scheduled but nothing inserted it"
    );
}

#[test]
fn the_context_menu_plugin_brings_the_focus_resource_it_dispatches_against() {
    let mut app = bare_app();
    app.add_plugins(tempera::context_menu::ContextMenuPlugin);

    assert!(
        app.world().get_resource::<InputFocus>().is_some(),
        "InputDispatchPlugin's systems read InputFocus; \
         only InputFocusPlugin inserts it"
    );
}

#[test]
fn a_bare_app_survives_frames_with_the_context_menu_plugin() {
    // The property the two assertions above stand in for. A resource can be
    // present and a system still fail validation on something else, so this
    // runs the schedule rather than inspecting the world.
    //
    // bevy's default error handler panics inside a task pool thread, which
    // surfaces here as the update itself panicking.
    let mut app = bare_app();
    app.add_plugins(tempera::context_menu::ContextMenuPlugin);

    for _ in 0..3 {
        app.update();
    }
}

#[test]
fn a_bare_app_survives_frames_with_the_whole_widget_set() {
    // `TemperaPlugin` is what a consumer actually adds, and it composes ~20
    // sub-plugins. Any one of them with a tempera-owned resource it forgot to
    // insert fails here.
    //
    // The bevy-stack plugins below are the price of running the real schedule
    // rather than a hand-built world, and each is here because a *third-party*
    // system reads something bevy owns:
    //
    // - `ImagePlugin` — `Assets<Image>`, written by the SVG rasteriser.
    // - `bevy_text::TextPlugin` — `Assets<Font>` and its `AssetEvent`s, read
    //   by `bevy_ui_text_input`'s pipeline.
    // - `PickingPlugin` + `InteractionPlugin` — `HoverMap`, read by text-input
    //   scroll handling. `PickingPlugin` alone is not enough: the resource is
    //   `InteractionPlugin`'s, the same split as `InputDispatchPlugin` and
    //   `InputFocusPlugin` above.
    //
    // None is a tempera defect, and pulling any of them into a widget plugin
    // would make every headless consumer pay for a renderer. Listing them here
    // is the honest form: this test says what a host owes, and the assertions
    // above say what tempera owes.
    let mut app = bare_app();
    app.add_plugins(bevy::text::TextPlugin)
        .add_plugins(bevy::picking::PickingPlugin)
        .add_plugins(bevy::picking::InteractionPlugin)
        .add_plugins(tempera::TemperaPlugin);

    for _ in 0..3 {
        app.update();
    }
}
