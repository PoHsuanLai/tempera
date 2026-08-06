//! A dimmed row recedes — all of it, in either theme.
//!
//! `TreeRowSpec::dimmed()` exists so a host can show a row the user cannot act
//! on rather than hiding it: a blacklisted plugin, a sample whose file has
//! gone, a device that unplugged. `tutti-plugin`'s catalog docs are the reason
//! hiding is not good enough — they call an un-blacklist affordance "not
//! optional", and nobody can un-blacklist a row that is not drawn.
//!
//! # Why these assert on resolved colour
//!
//! The interesting failure is not "the flag was ignored" — that would be
//! obvious. It is **dimming one part and not the others**, which reads as a
//! rendering fault rather than as a state, and which a test asserting only on
//! the label would pass over. So every case below checks the label, the suffix
//! and the icon together.
//!
//! And in both palettes, because a fixed grey is the mistake this is written
//! against: `hover_lift` moved a colour a constant amount and was correct on
//! dark and invisible on light. `ColorPalette::toward` takes the surface, so
//! the same call recedes downward on a dark page and upward on a light one.

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_resvg::prelude::{SvgColor, SvgFile, UiSvg};
use tempera::theme::{ColorPalette, ThemePlugin};
use tempera::tree_row::{TreeRowLabel, TreeRowSpec, TreeRowStyle, TreeRowSuffix, spawn_tree_row};

/// A headless app with the theme resources, in `palette`.
fn app(palette: ColorPalette) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .add_plugins(bevy::image::ImagePlugin::default())
        .add_plugins(bevy::text::TextPlugin)
        .add_plugins(ThemePlugin)
        // `TreeRowTokens` comes from `TreeRowPlugin`, which also brings picking
        // and the SVG loader. Inserted directly instead: these tests read
        // colours off a spawned row and never run a repaint system, so the rest
        // of that plugin would be setup for nothing.
        .init_resource::<tempera::tree_row::TreeRowTokens>()
        .insert_resource(palette);
    app
}

/// The three colours a row draws, in order: label, suffix, icon.
///
/// Every one is an `Option`, so a missing part fails the test that wanted it
/// rather than silently comparing nothing.
struct RowInk {
    label: Option<Color>,
    suffix: Option<Color>,
    icon: Option<Color>,
}

/// Spawn one row and read back what colour each part actually got.
fn ink(app: &mut App, spec: TreeRowSpec) -> RowInk {
    let world = app.world_mut();
    let mut state: SystemState<(Commands, TreeRowStyle)> = SystemState::new(world);
    let row = {
        let (mut commands, style) = state
            .get_mut(world)
            .expect("the theme resources this style needs are all inserted above");
        let row = spawn_tree_row(&mut commands, &style, spec);
        state.apply(world);
        row
    };

    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(row)
        .map(|c| c.iter().collect())
        .unwrap_or_default();

    let mut out = RowInk {
        label: None,
        suffix: None,
        icon: None,
    };
    for kid in kids {
        let world = app.world();
        if world.get::<TreeRowLabel>(kid).is_some() {
            out.label = world.get::<TextColor>(kid).map(|c| c.0);
        } else if world.get::<TreeRowSuffix>(kid).is_some() {
            out.suffix = world.get::<TextColor>(kid).map(|c| c.0);
        } else if world.get::<UiSvg>(kid).is_some() {
            out.icon = world.get::<SvgColor>(kid).map(|c| c.0);
        }
    }
    out
}

/// A full row: label, suffix and icon, so all three can be compared.
fn spec() -> TreeRowSpec {
    TreeRowSpec::new("FabFilter Pro-Q 3")
        .suffix("vst3")
        .icon(Handle::<SvgFile>::default())
}

/// Relative luminance — the same comparison `contrast.rs` uses internally,
/// restated here because it is not public.
fn luminance(c: Color) -> f32 {
    let s = c.to_srgba();
    let lin = |u: f32| {
        if u <= 0.040_45 {
            u / 12.92
        } else {
            ((u + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(s.red) + 0.7152 * lin(s.green) + 0.0722 * lin(s.blue)
}

#[test]
fn every_part_of_a_dimmed_row_recedes_in_both_themes() {
    // The property, stated over both shipped palettes rather than one. A fixed
    // grey passes on dark and fails here on light, which is exactly the
    // `hover_lift` bug that `toward` was written to avoid.
    for palette in [ColorPalette::dark(), ColorPalette::light()] {
        let bg = palette.background;
        let mut app = app(palette.clone());

        let plain = ink(&mut app, spec());
        let dim = ink(&mut app, spec().dimmed());

        let closer = |what: &str, before: Option<Color>, after: Option<Color>| {
            let before = before.unwrap_or_else(|| panic!("{what} was not drawn"));
            let after = after.unwrap_or_else(|| panic!("dimmed {what} was not drawn"));
            let d_before = (luminance(before) - luminance(bg)).abs();
            let d_after = (luminance(after) - luminance(bg)).abs();
            assert!(
                d_after < d_before,
                "{what} did not recede: separation {d_before} -> {d_after}"
            );
        };

        closer("label", plain.label, dim.label);
        closer("suffix", plain.suffix, dim.suffix);
        closer("icon", plain.icon, dim.icon);
    }
}

#[test]
fn an_undimmed_row_is_untouched() {
    // The other half: `dimmed()` must be opt-in, and a row that never asked
    // for it keeps the exact colours it had. Without this, dimming everything
    // unconditionally would still pass the test above.
    let palette = ColorPalette::dark();
    let mut app = app(palette.clone());
    let plain = ink(&mut app, spec());

    assert_eq!(plain.label, Some(palette.muted_foreground));
    assert_eq!(plain.suffix, Some(palette.muted_foreground));
    assert_eq!(plain.icon, Some(palette.muted_foreground));
}

#[test]
fn a_dimmed_header_is_still_brighter_than_a_dimmed_row() {
    // `dimmed` composes with `header` rather than replacing it. A group whose
    // contents are all unavailable is still a group, and flattening the two
    // would lose the hierarchy exactly where it is most needed.
    let palette = ColorPalette::dark();
    let mut app = app(palette);

    let header = ink(&mut app, spec().header().dimmed());
    let row = ink(&mut app, spec().dimmed());

    let (h, r) = (
        header.label.expect("header label"),
        row.label.expect("row label"),
    );
    assert!(
        luminance(h) > luminance(r),
        "a dimmed header must still outrank a dimmed row: {h:?} vs {r:?}"
    );
}

#[test]
fn a_dimmed_row_keeps_a_hosts_own_icon_tint_direction() {
    // A host that set `icon_tint` — the accent on a connected device, say —
    // still gets *its* colour dimmed, not the default one. Dimming the token
    // instead of the host's choice would silently discard the tint.
    let palette = ColorPalette::dark();
    let mut app = app(palette.clone());

    let tinted = ink(&mut app, spec().icon_tint(palette.destructive).dimmed());
    let default = ink(&mut app, spec().dimmed());

    let (t, d) = (tinted.icon.expect("icon"), default.icon.expect("icon"));
    assert_ne!(
        t, d,
        "a dimmed row discarded the host's icon tint and used the default"
    );

    // ...and the host's tint is itself dimmed, not passed through untouched.
    // Without this the test passes when *nothing* is dimmed, since two
    // undimmed icons also differ — which is how the first version of it
    // survived a mutation that dropped the icon dimming entirely.
    let undimmed = ink(&mut app, spec().icon_tint(palette.destructive));
    assert_ne!(
        t,
        undimmed.icon.expect("icon"),
        "the host's icon tint reached the row undimmed"
    );
}
