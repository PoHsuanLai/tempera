//! The tokens compile without a widget in sight.
//!
//! This crate exists because `tempera-dock` reads a `ColorPalette` and used to
//! compile the whole widget library — a text-input fork, an input manager, a
//! renderer — to get one. The split halved its dependency graph (1433 → 699
//! transitive crates).
//!
//! That property is a fact about the *manifest*, and a manifest edit can undo
//! it silently: adding `tempera-widgets` to this crate's dependencies would
//! restore the coupling and nothing would fail. This file is the guard. It is
//! an integration test, so it links against `tempera-theme` exactly the way an
//! outside consumer does — if the crate ever needs a widget to build, this
//! stops compiling.

use bevy::prelude::*;
use tempera_theme::{
    Base, ColorPalette, ControlSize, Density, Metrics, Scale, Spacing, Step, TextScale,
    ThemeConfig, ThemePlugin, Tokens, Typography,
};

#[test]
fn the_tokens_stand_up_without_a_widget_crate() {
    // Every public entry point, exercised from outside. If this file compiles,
    // the token layer is genuinely independent of the widget layer.
    let mut app = App::new();
    app.add_plugins(ThemePlugin);

    assert!(app.world().get_resource::<ColorPalette>().is_some());
    assert!(app.world().get_resource::<Spacing>().is_some());
    assert!(app.world().get_resource::<Typography>().is_some());
    assert!(app.world().get_resource::<Tokens>().is_some());
    assert!(app.world().get_resource::<ThemeConfig>().is_some());
}

#[test]
fn a_consumer_can_generate_its_own_scale() {
    // The shape a host uses to offer a density setting: pick a config, build,
    // and get told if the two inputs contradict.
    let config = ThemeConfig {
        base: Base::EIGHT,
        density: Density::Compact,
        text: TextScale::Small,
    };
    let tokens = config.build().expect("a coherent config");
    assert_eq!(tokens.scale.at(Step::BASE).get(), 8.0);
    assert_eq!(Scale::new(Base::FOUR).at(Step::BASE).get(), 4.0);
}

#[test]
fn metrics_reads_geometry_without_any_widget_type() {
    // `Metrics` is a `SystemParam`, so it can only be proven usable by running
    // it in a schedule. Doing that here — outside the widget crate — is what
    // shows the geometry half of the API is not secretly widget-coupled.
    let mut app = App::new();
    app.add_plugins(ThemePlugin);

    fn read_a_control_height(metrics: Metrics, mut out: ResMut<Captured>) {
        out.0 = metrics.control(ControlSize::Md).get();
        out.1 = metrics.gap(Step::new(2)).get();
    }

    #[derive(Resource, Default)]
    struct Captured(f32, f32);

    app.init_resource::<Captured>()
        .add_systems(Update, read_a_control_height);
    app.update();

    let got = app.world().resource::<Captured>();
    assert_eq!(got.0, 32.0, "the declared default control height");
    assert_eq!(got.1, 8.0, "step 2 on the base-4 scale");
}
