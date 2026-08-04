//! Where a declared tree actually puts its panes.
//!
//! Every other test in this crate asserts on the tree the dock *builds* —
//! which entities exist, which ids resolve, what `Val` each `Node` carries.
//! Those are the right level for structure, and they are blind to this:
//!
//! The root spawned with bevy's default `flex_direction: Row` while sizing its
//! child as a *column* child (`height: Px(0)` + `flex_grow`). In a row parent
//! `flex_grow` grows width, so the height stayed zero and **every pane below
//! the first fixed one collapsed to nothing**. The window rendered one strip
//! and blank space.
//!
//! Not one of the 132 tests noticed, because the declaration was correct at
//! every point they looked at. Only the resolved rectangle was wrong.
//!
//! So this file runs a real `bevy_ui` layout pass and reads `ComputedNode`.
//! Harness recipe is `tempera-widgets/tests/layout_containment.rs`.

use bevy::app::{HierarchyPropagatePlugin, PropagateSet, TaskPoolPlugin};
use bevy::camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::prelude::*;
use bevy::ui::ui_surface::UiSurface;
use bevy::ui::update::propagate_ui_target_cameras;
use bevy::ui::{ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiScale, ui_layout_system};
use tempera_dock::{Axis, DockLayout, DockTree, PaneRegistry, TemperaDockPlugin};

/// The viewport every test lays out in. Square, so a pane that borrows the
/// wrong axis's extent is not accidentally the right size.
const VIEW: f32 = 800.0;

/// The dock over a real layout pass, with no renderer and no window.
fn dock_app(layout: DockLayout) -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
        PostUpdate,
    ));
    app.insert_resource(layout);
    app.add_plugins(TemperaDockPlugin);
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
                    physical_size: UVec2::splat(VIEW as u32),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::splat(VIEW as u32),
                ..default()
            }),
            ..default()
        },
    ));
    // Two frames: the dock builds in `Update`, so the first layout pass runs
    // against a tree that does not exist yet.
    app.update();
    app.update();
    app
}

/// The laid-out size of the pane with this id.
fn size_of(app: &App, id: &str) -> Vec2 {
    let pane = app
        .world()
        .resource::<PaneRegistry>()
        .get(id)
        .unwrap_or_else(|| panic!("`{id}` was never built"));
    app.world()
        .get::<ComputedNode>(pane)
        .unwrap_or_else(|| panic!("`{id}` has no computed node"))
        .size()
}

/// The laid-out centre of the pane with this id.
fn center_of(app: &App, id: &str) -> Vec2 {
    let pane = app.world().resource::<PaneRegistry>().get(id).unwrap();
    app.world()
        .get::<bevy::ui::UiGlobalTransform>(pane)
        .unwrap()
        .translation
}

#[test]
fn a_lone_pane_fills_the_viewport() {
    let app = dock_app(DockLayout::new(DockTree::pane("only")));
    let size = size_of(&app, "only");
    assert!((size.x - VIEW).abs() < 0.5, "width {}", size.x);
    assert!((size.y - VIEW).abs() < 0.5, "height {}", size.y);
}

#[test]
fn a_column_split_gives_its_panes_height() {
    // The regression. Before the fix these came out `800 x 0` — full width,
    // no height at all — because the root laid out as a row while sizing its
    // child as a column child.
    let app = dock_app(DockLayout::new(DockTree::split(
        Axis::Column,
        [DockTree::pane("top"), DockTree::pane("bottom")],
    )));

    for id in ["top", "bottom"] {
        let size = size_of(&app, id);
        assert!(size.y > 1.0, "`{id}` has no height: {size:?}");
        assert!((size.x - VIEW).abs() < 0.5, "`{id}` width {}", size.x);
    }
    assert!(center_of(&app, "top").y < center_of(&app, "bottom").y);
}

#[test]
fn a_row_split_gives_its_panes_width() {
    // The other axis, which happened to work before the fix. Kept so a future
    // change that fixes columns by breaking rows is caught.
    let app = dock_app(DockLayout::new(DockTree::split(
        Axis::Row,
        [DockTree::pane("left"), DockTree::pane("right")],
    )));

    for id in ["left", "right"] {
        let size = size_of(&app, id);
        assert!(size.x > 1.0, "`{id}` has no width: {size:?}");
        assert!((size.y - VIEW).abs() < 0.5, "`{id}` height {}", size.y);
    }
    assert!(center_of(&app, "left").x < center_of(&app, "right").x);
}

#[test]
fn a_fixed_pane_keeps_its_size_and_the_rest_gets_the_remainder() {
    // The shape every app shell uses: a fixed bar, then everything else. This
    // is where the bug actually showed — the bar rendered and the rest of the
    // window was blank.
    const BAR: f32 = 40.0;
    let app = dock_app(DockLayout::new(DockTree::split(
        Axis::Column,
        [
            DockTree::pane("bar").fixed(BAR),
            DockTree::pane("body").flex(1.0),
        ],
    )));

    let bar = size_of(&app, "bar");
    let body = size_of(&app, "body");
    assert!((bar.y - BAR).abs() < 0.5, "bar height {}", bar.y);
    assert!(
        body.y > VIEW - BAR - 10.0,
        "the body must get what the bar did not, got {}",
        body.y
    );
}

#[test]
fn flex_weights_divide_the_axis_in_proportion() {
    // 1 : 6 : 1 across a row, which is the canonical shell layout.
    let app = dock_app(DockLayout::new(DockTree::split(
        Axis::Row,
        [
            DockTree::pane("a").flex(1.0),
            DockTree::pane("b").flex(6.0),
            DockTree::pane("c").flex(1.0),
        ],
    )));

    let (a, b, c) = (
        size_of(&app, "a").x,
        size_of(&app, "b").x,
        size_of(&app, "c").x,
    );
    assert!((a - c).abs() < 1.0, "equal weights: {a} vs {c}");
    assert!(b > a * 5.0, "6x weight should be far wider: {b} vs {a}");
    assert!(a + b + c <= VIEW + 1.0, "panes overflow the viewport");
}

#[test]
fn a_nested_split_lays_out_on_both_axes() {
    // A column holding a row — the shape that needs *both* axes correct at
    // once, and the one a shell actually declares.
    let app = dock_app(DockLayout::new(DockTree::split(
        Axis::Column,
        [
            DockTree::pane("bar").fixed(40.0),
            DockTree::split(Axis::Row, [DockTree::pane("left"), DockTree::pane("right")]).flex(1.0),
        ],
    )));

    for id in ["left", "right"] {
        let size = size_of(&app, id);
        assert!(size.x > 1.0, "`{id}` has no width: {size:?}");
        assert!(size.y > 1.0, "`{id}` has no height: {size:?}");
    }
    assert!(center_of(&app, "left").x < center_of(&app, "right").x);
    assert!(center_of(&app, "bar").y < center_of(&app, "left").y);
}
