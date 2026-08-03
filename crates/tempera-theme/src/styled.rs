//! Declaring geometry instead of computing it.
//!
//! # The problem with a per-widget metrics system
//!
//! The first conversion (`card`) wrote one system per widget, holding one
//! `&mut Node` query per *part* — root, header, body, chevron. Bevy proves
//! two mutable queries disjoint from their filters alone, so every query has
//! to exclude every other part's marker. Four parts means four exclusions
//! each; `command` has more, and the exclusion sets grow with the square of
//! the part count. Seventeen widgets of that is a lot of boilerplate whose
//! only job is to name the theme step a field should take.
//!
//! # Declared, resolved once
//!
//! [`StyledNode`] says what an entity's geometry *is*, in theme terms:
//!
//! ```ignore
//! StyledNode::new()
//!     .padding_x(Step::new(2))
//!     .height(ControlSize::Sm)
//!     .radius(Step::new(1))
//! ```
//!
//! One system resolves every one of them, so a widget adds a component at
//! spawn and writes no system at all. It is the same move the theme made for
//! colour: state the intent, let something else compute the pixels.
//!
//! # Only theme-derived fields
//!
//! `StyledNode` deliberately cannot express `width: 100%`, `flex_direction`,
//! or `position_type`. Those are *structure* — they never change with a
//! theme, and a widget spawning them directly is correct. Putting them here
//! would turn a geometry declaration into a second, worse `Node`.

use bevy::prelude::*;

use crate::config::Sizing;
use crate::metrics::ControlSize;
use crate::{Step, Tokens};

/// The theme-derived half of an entity's geometry, declared rather than
/// computed.
///
/// Every field is optional: a `None` leaves that `Node` field alone, so a
/// widget declares only what the theme owns and keeps structural values in
/// its own `Node`.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct StyledNode {
    /// Horizontal padding, at a step on the spacing scale.
    pub padding_x: Option<Step>,
    /// Vertical padding.
    pub padding_y: Option<Step>,
    /// Gap between children in a row.
    pub column_gap: Option<Step>,
    /// Gap between children in a column.
    pub row_gap: Option<Step>,
    /// Corner radius.
    pub radius: Option<Step>,
    /// Height, from the declared control sizes.
    pub height: Option<ControlSize>,
    /// A square box — width and height both — at a step on the scale.
    /// For icons and glyph slots.
    pub square: Option<Step>,
}

impl StyledNode {
    /// A declaration with nothing set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            padding_x: None,
            padding_y: None,
            column_gap: None,
            row_gap: None,
            radius: None,
            height: None,
            square: None,
        }
    }

    /// Horizontal padding at `step`.
    #[must_use]
    pub const fn padding_x(mut self, step: Step) -> Self {
        self.padding_x = Some(step);
        self
    }

    /// Vertical padding at `step`.
    #[must_use]
    pub const fn padding_y(mut self, step: Step) -> Self {
        self.padding_y = Some(step);
        self
    }

    /// Padding on both axes at `step`.
    #[must_use]
    pub const fn padding(self, step: Step) -> Self {
        self.padding_x(step).padding_y(step)
    }

    /// Row-direction gap at `step`.
    #[must_use]
    pub const fn column_gap(mut self, step: Step) -> Self {
        self.column_gap = Some(step);
        self
    }

    /// Column-direction gap at `step`.
    #[must_use]
    pub const fn row_gap(mut self, step: Step) -> Self {
        self.row_gap = Some(step);
        self
    }

    /// Corner radius at `step`.
    #[must_use]
    pub const fn radius(mut self, step: Step) -> Self {
        self.radius = Some(step);
        self
    }

    /// Height from a declared control size.
    #[must_use]
    pub const fn height(mut self, size: ControlSize) -> Self {
        self.height = Some(size);
        self
    }

    /// A square box at `step` — width and height together.
    #[must_use]
    pub const fn square(mut self, step: Step) -> Self {
        self.square = Some(step);
        self
    }
}

/// Resolve every [`StyledNode`] against the current theme.
///
/// Writes individual fields and compares before each, for two independent
/// reasons. `slider`, `switch` and `tabs` animate `left`/`width` every
/// frame, so a whole-`Node` write would fight the animation. And `bevy_ui`
/// gates its taffy upload on `Ref<Node>::is_changed()`, where a `DerefMut`
/// sets the flag whether or not the value moved — so an unconditional write
/// re-uploads the layout of every styled entity, every frame.
pub fn apply_styled_nodes(
    tokens: Res<Tokens>,
    // One query, not two. A `Changed<StyledNode>` filter does not make a
    // second `&mut Node` query disjoint from this one — Bevy proves
    // disjointness from component filters alone — so the "only the changed
    // ones" case is a per-entity test inside the loop instead.
    mut styled: Query<(Ref<StyledNode>, &mut Node)>,
) {
    let theme_moved = tokens.is_changed();
    for (style, node) in &mut styled {
        // A settled entity under an unchanged theme is skipped before the
        // `Mut<Node>` is ever touched, so it cannot be dirtied by accident.
        if theme_moved || style.is_changed() {
            apply(&tokens, &style, node);
        }
    }
}

fn apply(tokens: &Tokens, style: &StyledNode, mut node: Mut<Node>) {
    let gap = |s: Step| Val::Px(tokens.scale.at(s).get());

    if let Some(step) = style.padding_x {
        let want = gap(step);
        if node.padding.left != want || node.padding.right != want {
            node.padding.left = want;
            node.padding.right = want;
        }
    }
    if let Some(step) = style.padding_y {
        let want = gap(step);
        if node.padding.top != want || node.padding.bottom != want {
            node.padding.top = want;
            node.padding.bottom = want;
        }
    }
    if let Some(step) = style.column_gap {
        let want = gap(step);
        if node.column_gap != want {
            node.column_gap = want;
        }
    }
    if let Some(step) = style.row_gap {
        let want = gap(step);
        if node.row_gap != want {
            node.row_gap = want;
        }
    }
    if let Some(step) = style.radius {
        let want = BorderRadius::all(gap(step));
        if node.border_radius != want {
            node.border_radius = want;
        }
    }
    if let Some(size) = style.height {
        let want = Val::Px(Sizing::get(&tokens.sizing, size).get());
        if node.height != want {
            node.height = want;
        }
    }
    if let Some(step) = style.square {
        let want = gap(step);
        if node.width != want {
            node.width = want;
        }
        if node.height != want {
            node.height = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Base, Density, ThemeConfig, ThemePlugin};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(ThemePlugin);
        app
    }

    fn spawn(app: &mut App, style: StyledNode) -> Entity {
        app.world_mut().spawn((style, Node::default())).id()
    }

    #[test]
    fn a_declared_step_becomes_a_pixel_value() {
        let mut app = app();
        let e = spawn(&mut app, StyledNode::new().padding_x(Step::new(2)));
        app.update();

        let node = app.world().get::<Node>(e).unwrap();
        assert_eq!(node.padding.left, Val::Px(8.0));
        assert_eq!(node.padding.right, Val::Px(8.0));
        // Untouched, because the declaration said nothing about them.
        assert_eq!(node.padding.top, Val::Px(0.0));
    }

    #[test]
    fn an_undeclared_field_is_left_alone() {
        // The reason every field is an `Option`: a widget declares only what
        // the theme owns and keeps structural values — `width: 100%`, flex
        // direction — in its own `Node`. A `StyledNode` that wrote every
        // field would be a second, worse `Node`.
        let mut app = app();
        let e = app
            .world_mut()
            .spawn((
                StyledNode::new().radius(Step::new(1)),
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .id();
        app.update();

        let node = app.world().get::<Node>(e).unwrap();
        assert_eq!(node.width, Val::Percent(100.0), "structure survived");
        assert_eq!(node.flex_direction, FlexDirection::Column);
        assert_eq!(node.border_radius, BorderRadius::all(Val::Px(6.0)));
    }

    #[test]
    fn every_styled_node_follows_a_base_change() {
        // The property the whole move exists for: an entity already on
        // screen picks up a theme change, rather than keeping the geometry
        // it was spawned with.
        let mut app = app();
        let e = spawn(&mut app, StyledNode::new().padding_x(Step::new(2)));
        app.update();
        assert_eq!(
            app.world().get::<Node>(e).unwrap().padding.left,
            Val::Px(8.0)
        );

        let coarse = ThemeConfig {
            base: Base::EIGHT,
            ..default()
        };
        app.insert_resource(coarse)
            .insert_resource(coarse.build().expect("base 8 is coherent"));
        app.update();

        assert_eq!(
            app.world().get::<Node>(e).unwrap().padding.left,
            Val::Px(16.0),
            "a live entity followed the base"
        );
    }

    #[test]
    fn a_height_comes_from_the_declared_control_sizes() {
        let mut app = app();
        let e = spawn(&mut app, StyledNode::new().height(ControlSize::Sm));
        app.update();
        assert_eq!(app.world().get::<Node>(e).unwrap().height, Val::Px(28.0));

        let spacious = ThemeConfig {
            density: Density::Spacious,
            ..default()
        };
        app.insert_resource(spacious)
            .insert_resource(spacious.build().unwrap());
        app.update();
        assert_eq!(app.world().get::<Node>(e).unwrap().height, Val::Px(32.0));
    }

    #[test]
    fn an_idle_frame_dirties_nothing() {
        // `bevy_ui` gates its taffy upload on `Ref<Node>::is_changed()`, and
        // a `DerefMut` sets that flag whether or not the value moved — so an
        // unconditional write would re-upload every styled entity's layout
        // every frame. Counted inside the schedule, because change ticks
        // advance between frames and a check after `update()` always reads
        // clean regardless.
        #[derive(Resource, Default)]
        struct Dirtied(usize);

        fn count(mut d: ResMut<Dirtied>, nodes: Query<Ref<Node>>) {
            d.0 += nodes.iter().filter(|n| n.is_changed()).count();
        }

        let mut app = app();
        app.init_resource::<Dirtied>()
            .add_systems(Update, count.after(apply_styled_nodes));
        spawn(&mut app, StyledNode::new().padding(Step::new(2)));
        app.update();
        app.world_mut().resource_mut::<Dirtied>().0 = 0;

        // Re-insert an identical `Tokens`: the system runs again, but every
        // value it computes is the same.
        let same = *app.world().resource::<Tokens>();
        app.insert_resource(same);
        app.update();

        let touched = app.world().resource::<Dirtied>().0;
        assert_eq!(touched, 0, "a no-op recompute dirtied {touched} nodes");
    }

    #[test]
    fn a_square_sets_both_axes() {
        let mut app = app();
        let e = spawn(&mut app, StyledNode::new().square(Step::new(3)));
        app.update();
        let node = app.world().get::<Node>(e).unwrap();
        assert_eq!(node.width, Val::Px(12.0));
        assert_eq!(node.height, Val::Px(12.0));
    }
}
