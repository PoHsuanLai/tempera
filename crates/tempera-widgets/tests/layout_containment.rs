//! What a parent can guarantee about a child it does not control.
//!
//! A panel that hands out slots — an inspector stack, a toolbar, a settings
//! body — wants to promise that whatever a contributor puts in its slot cannot
//! wreck the panel around it. This file measures which parts of that promise
//! `bevy_ui` actually keeps, because the answer is not the obvious one and the
//! docs do not say it outright.
//!
//! The short version, measured below:
//!
//! - A child **cannot change the geometry of a node whose size is declared**. A
//!   9999px child leaves a fixed-size cell and row at exactly their declared
//!   sizes, and siblings do not move. This is the guarantee worth building on.
//! - **The declaration is the condition.** A cell that leaves its width to
//!   content sizing is inflated to 9999 by that same child. The *row* still
//!   holds, so a stack stays aligned regardless — two layers, only the outer
//!   one unconditional.
//! - A child **can position itself outside its parent's box**.
//!   `PositionType::Absolute` is relative to the parent's *origin*, not clamped
//!   to its bounds — so containment of *drawing* is a separate mechanism,
//!   `Overflow::clip()`, verified separately here.
//!
//! So a panel handing out slots owes its contributors two things: a row whose
//! height it declares, and a cell whose width it declares and clips. Neither is
//! optional, and neither is implied by the other.
//!
//! # Why this harness exists
//!
//! Nothing else in tempera runs a real layout pass. `page_strip`'s end caps,
//! the dock's dividers and every widget's `StyledNode` are checked
//! structurally — the right `Val` is on the right field — which cannot catch
//! "this arrangement does not do what the field values suggest". Standing up
//! `ui_layout_system` outside an `App` with rendering is fiddly enough
//! (`HierarchyPropagatePlugin` twice, `UiSurface`, three text resources, a
//! camera with a fabricated render target) that it is worth writing once.
//!
//! Recipe adapted from `bevy_ui`'s own `layout::tests`.

use bevy::app::{HierarchyPropagatePlugin, PropagateSet, TaskPoolPlugin};
use bevy::camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::prelude::*;
use bevy::ui::ui_surface::UiSurface;
use bevy::ui::update::propagate_ui_target_cameras;
use bevy::ui::{
    ComputedUiRenderTargetInfo, ComputedUiTargetCamera, OverflowClipMargin, UiGlobalTransform,
    UiScale, ui_layout_system,
};

const TARGET_WIDTH: u32 = 1000;
const TARGET_HEIGHT: u32 = 1000;

/// An app that runs a real `bevy_ui` layout pass, with no renderer.
fn layout_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
        PostUpdate,
    ));
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

    // A camera with a fabricated render target: layout needs a viewport to
    // resolve percentages against, and nothing here draws.
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                ..default()
            }),
            ..default()
        },
    ));
    app
}

fn size_of(app: &App, entity: Entity) -> Vec2 {
    app.world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size()
}

fn position_of(app: &App, entity: Entity) -> Vec2 {
    app.world()
        .get::<UiGlobalTransform>(entity)
        .expect("laid out")
        .translation
}

/// A fixed-size row holding a fixed-size cell, with `content` inside the cell.
///
/// The shape a slot-handing panel builds: the panel owns row and cell, the
/// contributor owns only what goes in.
fn row_cell_content(app: &mut App, cell_clips: bool, content: Node) -> (Entity, Entity, Entity) {
    let row = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(200.0),
            height: Val::Px(32.0),
            ..default()
        })
        .id();
    let cell = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(100.0),
                height: Val::Px(32.0),
                overflow: if cell_clips {
                    Overflow::clip()
                } else {
                    Overflow::visible()
                },
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    let inner = app.world_mut().spawn((content, ChildOf(cell))).id();
    app.update();
    (row, cell, inner)
}

#[test]
fn an_oversized_child_does_not_grow_its_cell_or_row() {
    // The guarantee a slot-handing panel needs. Whatever a contributor puts
    // in, the frame around it keeps the size the panel declared.
    let mut app = layout_app();
    let (row, cell, _) = row_cell_content(
        &mut app,
        true,
        Node {
            width: Val::Px(9999.0),
            height: Val::Px(9999.0),
            ..default()
        },
    );

    assert_eq!(size_of(&app, row), Vec2::new(200.0, 32.0), "row held");
    assert_eq!(size_of(&app, cell), Vec2::new(100.0, 32.0), "cell held");
}

#[test]
fn an_absolutely_positioned_child_does_not_move_its_cell_or_row() {
    // `Absolute` takes a child out of flow, which is the most likely thing a
    // contributor reaches for. It must not drag the frame with it.
    let mut app = layout_app();
    let (row, cell, _) = row_cell_content(
        &mut app,
        true,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(-500.0),
            top: Val::Px(-500.0),
            width: Val::Px(400.0),
            height: Val::Px(400.0),
            ..default()
        },
    );

    assert_eq!(size_of(&app, row), Vec2::new(200.0, 32.0));
    assert_eq!(size_of(&app, cell), Vec2::new(100.0, 32.0));
}

#[test]
fn a_hostile_child_does_not_displace_its_siblings() {
    // The failure this is all guarding against: one contributor's content
    // pushing every other row down the panel.
    let mut app = layout_app();
    let stack = app
        .world_mut()
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            width: Val::Px(200.0),
            ..default()
        })
        .id();

    let row = |app: &mut App, hostile: bool| {
        let row = app
            .world_mut()
            .spawn((
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                ChildOf(stack),
            ))
            .id();
        let cell = app
            .world_mut()
            .spawn((
                Node {
                    width: Val::Px(100.0),
                    height: Val::Px(32.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                ChildOf(row),
            ))
            .id();
        let content = if hostile {
            Node {
                width: Val::Px(9999.0),
                height: Val::Px(9999.0),
                ..default()
            }
        } else {
            Node {
                width: Val::Px(10.0),
                height: Val::Px(10.0),
                ..default()
            }
        };
        app.world_mut().spawn((content, ChildOf(cell)));
        row
    };

    let first = row(&mut app, false);
    let second = row(&mut app, true); // the bad one
    let third = row(&mut app, false);
    app.update();

    let ys: Vec<f32> = [first, second, third]
        .iter()
        .map(|e| position_of(&app, *e).y)
        .collect();

    assert_eq!(
        ys[1] - ys[0],
        32.0,
        "the row after a normal one sits exactly one row down"
    );
    assert_eq!(
        ys[2] - ys[1],
        32.0,
        "and so does the row after the hostile one: {ys:?}"
    );
}

#[test]
fn a_cell_without_a_declared_width_is_grown_by_its_child() {
    // The guarantee is **conditional**, and this is the condition. A cell that
    // leaves its width to content sizing inflates to fit — 9999px here — so a
    // panel that hands out slots must give every cell a width it chose itself.
    //
    // The row is the backstop: it holds at 200 either way, so the *stack* stays
    // aligned even when a cell is declared carelessly. Two layers, and only the
    // outer one is unconditional.
    let mut app = layout_app();
    let row = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(200.0),
            height: Val::Px(32.0),
            ..default()
        })
        .id();
    let cell = app
        .world_mut()
        .spawn((
            Node {
                // No width: content decides.
                height: Val::Px(32.0),
                overflow: Overflow::clip(),
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    app.world_mut().spawn((
        Node {
            width: Val::Px(9999.0),
            height: Val::Px(9999.0),
            ..default()
        },
        ChildOf(cell),
    ));
    app.update();

    assert_eq!(
        size_of(&app, cell).x,
        9999.0,
        "an unwidthed cell is sized by its content"
    );
    assert_eq!(
        size_of(&app, row),
        Vec2::new(200.0, 32.0),
        "but the row it sits in still holds, which is what keeps the stack aligned"
    );
}

#[test]
fn a_child_can_position_itself_outside_its_cell() {
    // Recorded because it is the *opposite* of what `PositionType::Absolute`'s
    // doc ("relative to its parent node") suggests on a first read. Relative
    // describes the origin, not a bound — nothing clamps a child to its
    // parent's box, so containment of *drawing* needs `Overflow::clip()` and
    // is a separate property from containment of *layout*.
    let mut app = layout_app();
    let (_, cell, content) = row_cell_content(
        &mut app,
        false,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(-500.0),
            width: Val::Px(50.0),
            height: Val::Px(50.0),
            ..default()
        },
    );

    let cell_left = position_of(&app, cell).x - size_of(&app, cell).x / 2.0;
    let content_left = position_of(&app, content).x - size_of(&app, content).x / 2.0;

    assert!(
        content_left < cell_left,
        "a child really does escape its parent's box: content_left={content_left}, \
         cell_left={cell_left}"
    );
}

#[test]
fn a_clipping_cell_bounds_what_its_child_may_draw() {
    // The other half of containment. Layout is held by the frame's own sizes;
    // drawing is held by this. A cell that forgets `Overflow::clip()` lets a
    // contributor paint over its neighbours.
    let mut app = layout_app();
    let (_, cell, _) = row_cell_content(
        &mut app,
        true,
        Node {
            width: Val::Px(9999.0),
            height: Val::Px(9999.0),
            ..default()
        },
    );

    let computed = app.world().get::<ComputedNode>(cell).expect("laid out");
    let clip = computed.resolve_clip_rect(Overflow::clip(), OverflowClipMargin::default());

    assert_eq!(
        clip.size(),
        Vec2::new(100.0, 32.0),
        "the clip rect is the cell's own box, so nothing inside paints past it"
    );
}

#[test]
fn a_visible_cell_bounds_nothing() {
    // The contrast that makes the previous test mean something: without
    // `clip()`, the same cell imposes no bound at all, so a panel that hands
    // out slots must set it rather than assume it.
    let mut app = layout_app();
    let (_, cell, _) = row_cell_content(
        &mut app,
        false,
        Node {
            width: Val::Px(9999.0),
            height: Val::Px(9999.0),
            ..default()
        },
    );

    let computed = app.world().get::<ComputedNode>(cell).expect("laid out");
    let clip = computed.resolve_clip_rect(Overflow::visible(), OverflowClipMargin::default());

    assert!(
        clip.size().x > 100.0,
        "an unclipped cell does not bound its children: {:?}",
        clip.size()
    );
}
