//! Where a spawned button lands inside its parent, on the cross axis.
//!
//! `spawn_button_sized` writes `align_self` on every button it makes, and that
//! one property means two different things depending on the parent's
//! direction. In a **column** it decides horizontal *size* — without it,
//! `align_items: Stretch` (bevy's default) blows an intrinsic-width button out
//! to the full row width, which is the case the value was chosen for. In a
//! **row** the same property decides vertical *position*, and it overrides
//! whatever the parent asked for.
//!
//! `FlexStart` answers the first question and gets the second wrong: it pins
//! the button to the top edge even when the parent set `align_items: Center`
//! precisely to avoid that. A 24px icon button in a 40px toolbar sat 8px high,
//! with the row itself already correct.
//!
//! `Center` answers both. These tests hold it to that in each parent, because
//! fixing one direction by breaking the other is the failure mode here.
//!
//! Harness recipe is `layout_containment.rs`'s.

use bevy::app::{HierarchyPropagatePlugin, PropagateSet, TaskPoolPlugin};
use bevy::camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::ui::ui_surface::UiSurface;
use bevy::ui::update::propagate_ui_target_cameras;
use bevy::ui::{
    ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiGlobalTransform, UiScale,
    ui_layout_system,
};
use tempera::prelude::{ButtonContent, ButtonSize, ButtonVariant, spawn_button_sized};

const TARGET: u32 = 1000;

/// The toolbar case, in the sizes dawai's title bar actually uses.
const BAR_HEIGHT: f32 = 40.0;

fn layout_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
        PostUpdate,
    ));
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::image::ImagePlugin::default());
    app.add_plugins(tempera::theme::ThemePlugin);
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
                    physical_size: UVec2::splat(TARGET),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::splat(TARGET),
                ..default()
            }),
            ..default()
        },
    ));
    app
}

/// Spawn a button inside a parent laid out along `direction`, and run layout.
fn button_in(
    app: &mut App,
    direction: FlexDirection,
    parent: Node,
    size: ButtonSize,
) -> (Entity, Entity) {
    let mut parent_node = parent;
    parent_node.flex_direction = direction;
    let container = app.world_mut().spawn(parent_node).id();

    let button = app.world_mut().run_system_once(
        move |mut commands: Commands, style: tempera::prelude::ButtonStyle| {
            spawn_button_sized(
                &mut commands,
                &style,
                ButtonContent::text("x"),
                ButtonVariant::Ghost,
                size,
            )
        },
    );
    let button = button.expect("spawn succeeded");
    app.world_mut()
        .entity_mut(button)
        .insert(ChildOf(container));
    app.update();
    (container, button)
}

fn center_y(app: &App, e: Entity) -> f32 {
    app.world()
        .get::<UiGlobalTransform>(e)
        .unwrap()
        .translation
        .y
}

fn size(app: &App, e: Entity) -> Vec2 {
    app.world().get::<ComputedNode>(e).unwrap().size()
}

#[test]
fn a_button_in_a_centring_row_is_vertically_centred() {
    // The bug this file exists for. The parent declares `align_items: Center`;
    // the button must not override it and ride the top edge.
    let mut app = layout_app();
    let (row, button) = button_in(
        &mut app,
        FlexDirection::Row,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(BAR_HEIGHT),
            align_items: AlignItems::Center,
            ..default()
        },
        ButtonSize::IconSm,
    );

    assert!(
        (center_y(&app, button) - center_y(&app, row)).abs() < 0.5,
        "button centre {} is not the row's {} — a 24px control in a {BAR_HEIGHT}px bar",
        center_y(&app, button),
        center_y(&app, row),
    );
}

#[test]
fn a_button_in_a_column_keeps_its_intrinsic_width() {
    // The case the original `FlexStart` was chosen for, and which the fix must
    // not regress: bevy's default `align_items: Stretch` on a column parent
    // would blow an intrinsic-width button out to the parent's full width.
    //
    // A **text** button, deliberately. An icon button sets an explicit square
    // `width`, so it cannot stretch whatever `align_self` says — writing this
    // with `IconSm` produced a test that passed with the property removed
    // entirely, which is no test at all.
    let mut app = layout_app();
    let (column, button) = button_in(
        &mut app,
        FlexDirection::Column,
        Node {
            width: Val::Px(400.0),
            height: Val::Px(200.0),
            ..default()
        },
        ButtonSize::Md,
    );

    let w = size(&app, button).x;
    assert!(
        w < size(&app, column).x,
        "button stretched to its column's full width ({w})"
    );
}

#[test]
fn a_button_in_a_bare_row_still_does_not_stretch_vertically() {
    // No `align_items` on the parent at all, so bevy's `Stretch` default
    // applies — on a row that is the *vertical* axis. The button's own
    // `align_self` is what keeps it 24px tall in a 40px bar rather than being
    // stretched to fill it.
    let mut app = layout_app();
    let (_row, button) = button_in(
        &mut app,
        FlexDirection::Row,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(BAR_HEIGHT),
            ..default()
        },
        ButtonSize::IconSm,
    );

    assert!(
        size(&app, button).y < BAR_HEIGHT,
        "button stretched to the full bar height ({})",
        size(&app, button).y
    );
}
