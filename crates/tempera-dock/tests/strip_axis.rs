//! What a strip promises on either axis, measured rather than declared.
//!
//! `page_strip.rs`'s own tests assert on `Node` **field values** — the radius
//! written, the chip spawned. That is the right level for structure, and it is
//! blind to the failure this file exists for.
//!
//! A chip declaring `height: Px(24)` inside a *row* is 24 tall and shares the
//! width. The identical chip inside a *column* is 24 **long** and spans the
//! full width, because `height` moved from the cross axis to the main one.
//! Nothing errors. The declared value is still 24, so every assertion on a
//! `Val` still passes — the strip is simply the wrong shape once laid out.
//!
//! That is why these run a real `bevy_ui` layout pass and read `ComputedNode`.
//! The harness recipe comes from `tempera-widgets/tests/layout_containment.rs`.
//!
//! # The property, not the spelling
//!
//! Each test states the same requirement in axis-neutral terms: chips share
//! the strip's **length** equally and each is the declared **thickness**
//! across. Swapping the implementation for any other spelling that satisfies
//! that — `width: 100%` on a column, say — leaves these passing, and rightly
//! so. What breaks them is a chip whose size depends on which axis nobody
//! remembered to branch on.

use bevy::app::{HierarchyPropagatePlugin, PropagateSet, TaskPoolPlugin};
use bevy::camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::prelude::*;
use bevy::ui::ui_surface::UiSurface;
use bevy::ui::update::propagate_ui_target_cameras;
use bevy::ui::{ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiScale, ui_layout_system};
use tempera_dock::page::{ActivePage, Page, PageId, PageOrder};
use tempera_dock::page_strip::{PageStrip, PageStripStyle};
use tempera_dock::{Axis, TemperaDockPlugin};
use tempera_theme::ColorPalette;

/// Side of the square strip every test lays out in.
const STRIP: f32 = 300.0;
/// Chip thickness these strips declare.
const THICK: f32 = 24.0;

/// An app that runs a real layout pass, with no renderer.
fn layout_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
        PostUpdate,
    ));
    app.add_plugins(TemperaDockPlugin);
    app.init_resource::<ColorPalette>();
    app.init_resource::<UiScale>();
    app.init_resource::<UiSurface>();
    app.init_resource::<bevy::text::TextPipeline>();
    app.init_resource::<bevy::text::FontCx>();
    app.init_resource::<bevy::text::ScaleCx>();
    app.init_resource::<bevy::transform::StaticTransformOptimizations>();
    app.add_systems(
        PostUpdate,
        (ApplyDeferred, propagate_ui_target_cameras, ui_layout_system).chain(),
    );
    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default()
            .after(propagate_ui_target_cameras)
            .before(ui_layout_system),
    );
    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiRenderTargetInfo>::default()
            .after(propagate_ui_target_cameras)
            .before(ui_layout_system),
    );
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1000, 1000),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::new(1000, 1000),
                ..default()
            }),
            ..default()
        },
    ));
    app
}

/// A square strip on `axis` with `n` pages, laid out.
///
/// The strip is square so neither axis can pass by borrowing the other's
/// extent: 300×300 means "shares the length" and "spans the width" are
/// different numbers on both axes.
fn strip_of(axis: Axis, n: usize) -> (App, Entity) {
    let mut app = layout_app();
    let pane = app.world_mut().spawn(ActivePage::none()).id();
    for i in 0..n {
        app.world_mut().spawn((
            Page,
            PageId::from(format!("page{i}")),
            PageOrder(i as i32 * 10),
            ChildOf(pane),
        ));
    }
    let strip = app
        .world_mut()
        .spawn((
            PageStrip(pane),
            PageStripStyle {
                axis,
                thickness: THICK,
                ..default()
            },
            Node {
                width: Val::Px(STRIP),
                height: Val::Px(STRIP),
                ..default()
            },
        ))
        .id();
    app.update();
    (app, strip)
}

/// The laid-out size of every chip in the strip, in order.
fn chip_sizes(app: &App, strip: Entity) -> Vec<Vec2> {
    app.world()
        .get::<Children>(strip)
        .map(|kids| {
            kids.iter()
                .filter_map(|k| app.world().get::<ComputedNode>(k))
                .map(|c| c.size())
                .collect()
        })
        .unwrap_or_default()
}

/// The chip's extent along `axis` (its length) and across it (its thickness).
fn along_and_across(axis: Axis, size: Vec2) -> (f32, f32) {
    (axis.extent_of(size), axis.cross().extent_of(size))
}

#[test]
fn chips_share_the_length_and_keep_their_thickness_on_a_row() {
    let (app, strip) = strip_of(Axis::Row, 3);
    let sizes = chip_sizes(&app, strip);
    assert_eq!(sizes.len(), 3);
    for size in sizes {
        let (along, across) = along_and_across(Axis::Row, size);
        assert!(
            (along - STRIP / 3.0).abs() < 0.5,
            "each chip takes a third of the length, got {along}"
        );
        assert!(
            (across - THICK).abs() < 0.5,
            "and is the declared thickness across, got {across}"
        );
    }
}

#[test]
fn chips_share_the_length_and_keep_their_thickness_on_a_column() {
    // The same requirement, stated identically. This is the test the original
    // implementation would have failed: its chips came out 300 wide, not 24.
    let (app, strip) = strip_of(Axis::Column, 3);
    let sizes = chip_sizes(&app, strip);
    assert_eq!(sizes.len(), 3);
    for size in sizes {
        let (along, across) = along_and_across(Axis::Column, size);
        assert!(
            (along - STRIP / 3.0).abs() < 0.5,
            "each chip takes a third of the length, got {along}"
        );
        assert!(
            (across - THICK).abs() < 0.5,
            "and is the declared thickness across, got {across}"
        );
    }
}

#[test]
fn a_column_stacks_its_chips_and_a_row_lines_them_up() {
    // Sharing the length is not enough on its own: three chips could each be a
    // third the length and still sit on top of one another. This pins that
    // they advance along the axis, and only along it.
    for axis in [Axis::Row, Axis::Column] {
        let (app, strip) = strip_of(axis, 3);
        let kids: Vec<Entity> = app.world().get::<Children>(strip).unwrap().iter().collect();
        let positions: Vec<Vec2> = kids
            .iter()
            .map(|k| {
                app.world()
                    .get::<bevy::ui::UiGlobalTransform>(*k)
                    .unwrap()
                    .translation
            })
            .collect();

        for pair in positions.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(
                axis.extent_of(b) > axis.extent_of(a),
                "{axis:?}: chips must advance along the axis"
            );
            assert!(
                (axis.cross().extent_of(b) - axis.cross().extent_of(a)).abs() < 0.5,
                "{axis:?}: and must not drift across it"
            );
        }
    }
}

#[test]
fn a_lone_chip_fills_the_length_on_either_axis() {
    for axis in [Axis::Row, Axis::Column] {
        let (app, strip) = strip_of(axis, 1);
        let sizes = chip_sizes(&app, strip);
        assert_eq!(sizes.len(), 1, "{axis:?}");
        let (along, across) = along_and_across(axis, sizes[0]);
        assert!((along - STRIP).abs() < 0.5, "{axis:?}: got {along}");
        assert!((across - THICK).abs() < 0.5, "{axis:?}: got {across}");
    }
}

#[test]
fn a_strip_declaring_no_axis_lays_out_as_a_row() {
    // `PageStripStyle::default()` is `Axis::Row`, so a host that inserts
    // nothing gets the horizontal switcher it had before this was a choice.
    let mut app = layout_app();
    let pane = app.world_mut().spawn(ActivePage::none()).id();
    for i in 0..2 {
        app.world_mut().spawn((
            Page,
            PageId::from(format!("page{i}")),
            PageOrder(i * 10),
            ChildOf(pane),
        ));
    }
    let strip = app
        .world_mut()
        .spawn((
            PageStrip(pane),
            Node {
                width: Val::Px(STRIP),
                height: Val::Px(STRIP),
                ..default()
            },
        ))
        .id();
    app.update();

    let sizes = chip_sizes(&app, strip);
    assert_eq!(sizes.len(), 2);
    let (along, _) = along_and_across(Axis::Row, sizes[0]);
    assert!(
        (along - STRIP / 2.0).abs() < 0.5,
        "an unstyled strip must still run left-to-right, got {along}"
    );
}

#[test]
fn a_strip_that_declares_no_size_still_spans_its_container() {
    // Every test above hands the strip an explicit `width`/`height`, which is
    // what a test writes and *not* what a host writes: a host spawns
    // `PageStrip(pane)` and lets `#[require(Node)]` supply the rest. That node
    // is `width: auto`.
    //
    // Chips are `flex_grow: 1` over `flex_basis: 0`, which splits the main axis
    // evenly — but only when there is a main axis to split. Against an `auto`
    // root the strip shrink-wraps, every chip collapses to its label width, and
    // the segmented control renders as a couple of loose buttons hugging one
    // end of the pane. Nothing errors, and every assertion in this file still
    // passed, because they all supplied the size the bug was about.
    //
    // Reported from the assembled application: "it's supposed to be centered".
    const BOX: f32 = 400.0;

    let mut app = layout_app();
    let pane = app.world_mut().spawn(ActivePage::none()).id();
    for i in 0..2 {
        app.world_mut().spawn((
            Page,
            PageId::from(format!("page{i}")),
            PageOrder(i * 10),
            ChildOf(pane),
        ));
    }

    // A container of a known width, and a strip inside it that declares
    // nothing — the shape a host actually produces.
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(BOX),
            height: Val::Px(BOX),
            ..default()
        })
        .id();
    let strip = app.world_mut().spawn((PageStrip(pane), ChildOf(host))).id();
    app.update();

    let width = app
        .world()
        .get::<ComputedNode>(strip)
        .expect("the strip is laid out")
        .size()
        .x;
    assert!(
        (width - BOX).abs() < 0.5,
        "a row strip must span its container: got {width} of {BOX}"
    );

    // And the chips divide that span rather than hugging their text.
    let sizes = chip_sizes(&app, strip);
    assert_eq!(sizes.len(), 2);
    assert!(
        (sizes[0].x - sizes[1].x).abs() < 0.5,
        "chips must share the length equally, got {:?}",
        sizes.iter().map(|s| s.x).collect::<Vec<_>>()
    );
    assert!(
        sizes[0].x > BOX / 4.0,
        "each chip should take about half the strip, got {}",
        sizes[0].x
    );
}
