//! The promise [`add_row`] makes: a section cannot alter the panel around it.
//!
//! `section.rs` asserts the *structure* — which frames exist, in what order,
//! which are gated out. This asserts the **geometry**, by running a real
//! `bevy_ui` layout pass over a stack whose sections behave badly.
//!
//! The properties come from `tempera-widgets/tests/layout_containment.rs`,
//! which measured what `bevy_ui` will and will not guarantee. This file checks
//! that `add_row` actually applies them: a declared row height, a declared cell
//! width, and `Overflow::clip()` on every cell. Each is an obligation rather
//! than a preference — drop any one and a section can push its neighbours
//! around.
//!
//! # What these pin, and what they do not
//!
//! The *properties*, not the spelling. Swapping the cell's `flex_basis: 0` +
//! `flex_grow: 1` for `width: 100%` leaves every one of them passing, and
//! rightly so — both take the width from the row rather than from the content,
//! which is the whole requirement. What breaks them is a cell that sizes itself
//! *around* whatever a section put in, which is the failure the layout tests
//! identified and the reason any width is declared at all.

use bevy::app::{HierarchyPropagatePlugin, PropagateSet, TaskPoolPlugin};
use bevy::camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::prelude::*;
use bevy::ui::ui_surface::UiSurface;
use bevy::ui::update::propagate_ui_target_cameras;
use bevy::ui::{
    ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiGlobalTransform, UiScale,
    ui_layout_system,
};
use tempera_dock::section::{Row, SectionCell, SectionRow, SectionStyle, add_row};
use tempera_theme::{ColorPalette, Metrics, ThemePlugin};

/// An app that runs a real layout pass, with no renderer.
///
/// Same recipe as `tempera-widgets`' containment tests; see the note there on
/// why standing this up is worth doing rather than asserting on `Val`s.
fn layout_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
        PostUpdate,
    ));
    app.add_plugins(ThemePlugin);
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

/// Content a badly-behaved section might spawn: far too big, out of flow.
fn hostile() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(-400.0),
        width: Val::Px(9999.0),
        height: Val::Px(9999.0),
        ..default()
    }
}

/// Build a body with `rows` rows, filling the nth with `content`.
fn body_with_rows(app: &mut App, rows: &[Row], hostile_at: Option<usize>) -> (Entity, Vec<Entity>) {
    let body = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(240.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();

    let mut state: bevy::ecs::system::SystemState<(Commands, Metrics, Res<ColorPalette>)> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let cells: Vec<Entity> = {
        let (mut commands, metrics, palette) = state.get(app.world()).expect("theme present");
        let style = SectionStyle::default();
        rows.iter()
            .map(|row| add_row(&mut commands, &metrics, &palette, &style, body, *row))
            .collect()
    };
    state.apply(app.world_mut());

    if let Some(index) = hostile_at {
        app.world_mut().spawn((hostile(), ChildOf(cells[index])));
    }
    app.update();
    (body, cells)
}

fn size_of(app: &App, entity: Entity) -> Vec2 {
    app.world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size()
}

fn y_of(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<UiGlobalTransform>(entity)
        .expect("laid out")
        .translation
        .y
}

fn rows_of(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<SectionRow>>()
        .iter(app.world())
        .collect()
}

#[test]
fn a_hostile_section_does_not_displace_the_rows_below_it() {
    // The whole promise. A section that spawns something enormous must not
    // push its neighbours down the panel.
    let mut app = layout_app();
    let rows = [
        Row::labelled("First"),
        Row::labelled("Second"),
        Row::labelled("Third"),
    ];
    body_with_rows(&mut app, &rows, Some(1));

    let mut ys: Vec<f32> = rows_of(&mut app).iter().map(|r| y_of(&app, *r)).collect();
    ys.sort_by(f32::total_cmp);

    let first_gap = ys[1] - ys[0];
    let second_gap = ys[2] - ys[1];
    assert_eq!(
        first_gap, second_gap,
        "the row after the hostile one sits the same distance down as any \
         other: {ys:?}"
    );
}

#[test]
fn a_control_row_keeps_its_declared_height() {
    // Compared against the *same row with harmless content*, not against a
    // loose bound: a row that grew from 24px to 60 is broken even though 60
    // is nowhere near 9999, and an upper bound would call that a pass.
    let mut app = layout_app();
    body_with_rows(&mut app, &[Row::labelled("Volume")], None);
    let calm_row = rows_of(&mut app)[0];
    let calm = size_of(&app, calm_row).y;

    let mut app = layout_app();
    body_with_rows(&mut app, &[Row::labelled("Volume")], Some(0));
    let hostile_row = rows_of(&mut app)[0];
    let hostile = size_of(&app, hostile_row).y;

    assert_eq!(
        hostile, calm,
        "a 9999px child must leave its row exactly as declared"
    );
    assert!(calm > 0.0, "and the declared height is not zero");
}

#[test]
fn a_cell_keeps_its_width() {
    // The condition the layout tests turned up: a cell that leaves its width
    // to content sizing gets inflated. `add_row` must take the width from the
    // row instead — `flex_basis: 0` plus `flex_grow`, never a width sized
    // around the content.
    let mut app = layout_app();
    let (_, calm_cells) = body_with_rows(&mut app, &[Row::labelled("Volume")], None);
    let calm_row = rows_of(&mut app)[0];
    let calm_row_width = size_of(&app, calm_row).x;
    let calm_cell = size_of(&app, calm_cells[0]).x;

    let mut app = layout_app();
    let (_, cells) = body_with_rows(&mut app, &[Row::labelled("Volume")], Some(0));
    let hostile_cell = size_of(&app, cells[0]).x;

    assert_eq!(
        hostile_cell, calm_cell,
        "a 9999px child must leave the cell exactly the width the row gave it"
    );
    assert!(
        calm_cell > 0.0 && calm_cell < calm_row_width,
        "and that width is the row's leftover, not zero and not the whole row: \
         cell={calm_cell}, row={calm_row_width}"
    );
}

#[test]
fn every_cell_clips() {
    // Containment of *drawing* is a separate mechanism from containment of
    // layout, and neither implies the other. A cell that forgets this lets a
    // section paint over its neighbours.
    let mut app = layout_app();
    let (_, cells) = body_with_rows(
        &mut app,
        &[Row::labelled("Volume"), Row::full_width()],
        None,
    );

    for cell in cells {
        let node = app.world().get::<Node>(cell).expect("spawned");
        assert_eq!(
            node.overflow,
            Overflow::clip(),
            "a section's cell must clip what it draws"
        );
    }
}

#[test]
fn a_full_width_row_gives_its_cell_the_whole_row() {
    // The matrix case: no label column, so the content gets the full width —
    // still one row, still contained.
    let mut app = layout_app();
    let (_, labelled) = body_with_rows(&mut app, &[Row::labelled("Volume")], None);
    let labelled_width = size_of(&app, labelled[0]).x;

    let mut app = layout_app();
    let (_, full) = body_with_rows(&mut app, &[Row::full_width()], None);
    let full_width = size_of(&app, full[0]).x;

    assert!(
        full_width > labelled_width,
        "an unlabelled row should give its cell more room: \
         full={full_width}, labelled={labelled_width}"
    );
}

#[test]
fn a_growing_row_is_sized_by_its_content() {
    // The deliberate opt-out. A modulation matrix cannot state a height up
    // front, so `grows()` lets the row follow its content — and that is the
    // one property such a row gives up.
    let mut app = layout_app();
    let body = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(240.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();

    let mut state: bevy::ecs::system::SystemState<(Commands, Metrics, Res<ColorPalette>)> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let cell = {
        let (mut commands, metrics, palette) = state.get(app.world()).expect("theme present");
        add_row(
            &mut commands,
            &metrics,
            &palette,
            &SectionStyle::default(),
            body,
            Row::full_width().grows(),
        )
    };
    state.apply(app.world_mut());

    app.world_mut().spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(300.0),
            ..default()
        },
        ChildOf(cell),
    ));
    app.update();

    let row = rows_of(&mut app)[0];
    assert!(
        size_of(&app, row).y >= 300.0,
        "a growing row follows its content: {:?}",
        size_of(&app, row)
    );
}

#[test]
fn a_growing_row_still_clips_sideways() {
    // What `grows()` does *not* give up. Height follows content; width does
    // not, so a matrix cannot spill into the panel beside it.
    let mut app = layout_app();
    let body = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(240.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();

    let mut state: bevy::ecs::system::SystemState<(Commands, Metrics, Res<ColorPalette>)> =
        bevy::ecs::system::SystemState::new(app.world_mut());
    let cell = {
        let (mut commands, metrics, palette) = state.get(app.world()).expect("theme present");
        add_row(
            &mut commands,
            &metrics,
            &palette,
            &SectionStyle::default(),
            body,
            Row::full_width().grows(),
        )
    };
    state.apply(app.world_mut());

    app.world_mut().spawn((hostile(), ChildOf(cell)));
    app.update();

    assert!(
        size_of(&app, cell).x <= 240.0,
        "width is still declared even when height is not: {:?}",
        size_of(&app, cell)
    );
    assert_eq!(
        app.world().get::<Node>(cell).unwrap().overflow,
        Overflow::clip(),
        "and it still clips"
    );
}

#[test]
fn a_section_never_receives_its_row_or_its_cell() {
    // The structural half of the guarantee: `add_row` hands back the entity to
    // fill, and neither the row nor the cell is reachable from it. A section
    // holding only this entity has no path to the geometry around it.
    let mut app = layout_app();
    let (_, cells) = body_with_rows(&mut app, &[Row::labelled("Volume")], None);
    let handed_back = cells[0];

    assert!(
        app.world().get::<SectionCell>(handed_back).is_some(),
        "the returned entity is the cell itself, which the section fills"
    );
    assert!(
        app.world().get::<SectionRow>(handed_back).is_none(),
        "and it is never the row"
    );
}
