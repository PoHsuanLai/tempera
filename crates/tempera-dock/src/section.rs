//! A stack of collapsible sections, contributed by crates that have never
//! heard of each other.
//!
//! An inspector, a properties panel, a settings sidebar: a column of cards,
//! each owned by whichever crate knows about that subject. A section is an
//! entity, so a crate that is not compiled in contributes nothing and the
//! stack simply has one fewer card — no feature flag, no `cfg`, nothing naming
//! it in the shell.
//!
//! ```
//! use bevy::prelude::*;
//! use tempera_dock::{Section, SectionId, SectionLabel, SectionOrder};
//!
//! fn declare(mut commands: Commands, pane: Entity) {
//!     commands.spawn((
//!         Section,
//!         SectionId::from("transport"),
//!         SectionLabel("TRANSPORT"),
//!         SectionOrder(10),
//!         ChildOf(pane),
//!     ));
//! }
//! ```
//!
//! # Layout is the stack's; content is the section's
//!
//! This is the whole point, and it is enforced structurally rather than by
//! convention. A section never receives the row it occupies or the cell inside
//! it — [`add_row`] hands back only the innermost entity. Row height, the
//! label column's width, gaps and clipping all live on nodes the section
//! cannot reach, so it cannot indent itself differently, invent its own
//! spacing, or push its neighbours around.
//!
//! What goes *in* the cell is unconstrained: a slider, a bespoke meter, a
//! whole subtree. The stack has no opinion and needs none.
//!
//! # What that guarantee rests on
//!
//! Measured in `tempera-widgets/tests/layout_containment.rs`, because the
//! answer is not the obvious one:
//!
//! - A child cannot resize a node **whose size is declared**. A 9999px child
//!   leaves a fixed row and cell exactly as declared, and the rows below it do
//!   not shift.
//! - **The declaration is the condition.** A cell that leaves its width to
//!   content sizing is inflated by that same child. Every cell here declares
//!   one.
//! - `PositionType::Absolute` is relative to the parent's *origin*, not
//!   clamped to its box, so containing what a child **draws** is a separate
//!   mechanism. Every cell here sets `Overflow::clip()`.
//!
//! # Gating
//!
//! A section that only applies sometimes carries a
//! [`When`](tempera_input::When) — the same condition a command uses to decide
//! whether it applies and a menu row uses to decide whether it shows:
//!
//! ```ignore
//! commands.spawn((
//!     Section,
//!     SectionId::from("note"),
//!     SectionLabel("NOTE"),
//!     SectionOrder(60),
//!     When::new(page_is_active("symbolic")),
//!     ChildOf(pane),
//! ));
//! ```
//!
//! The implementation this replaces spawned every section unconditionally and
//! let each decide internally whether to look empty, with a hand-written match
//! over content ids living in the shell. A gate on the section that cares
//! deletes both.

use bevy::prelude::*;
use tempera_theme::{ColorPalette, ControlSize, Metrics, Step, StyledNode};

/// One section of a stacked panel. Spawn as a child of the pane it belongs to.
///
/// Inserting it attaches [`SectionId`], [`SectionLabel`] and [`SectionOrder`]
/// at their defaults through Bevy's required-components chain. Override at
/// minimum [`SectionId`] — the default empty string names nothing.
#[derive(Component, Debug, Clone, Copy, Default)]
#[require(SectionId, SectionLabel, SectionOrder)]
pub struct Section;

/// Stable identity for a section, unique within its stack.
///
/// Owned rather than `&'static str`, matching [`PaneId`](crate::PaneId) and
/// [`PageId`](crate::PageId): an extension can mint one at runtime. It is also
/// how a section finds its own [`SectionBody`] to fill.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SectionId(pub String);

impl SectionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for SectionId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// The heading shown on the section's card.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SectionLabel(pub &'static str);

/// Sort key within the stack; lower goes first. Ties break on [`SectionId`].
///
/// A number rather than declaration order, because sections are declared by
/// whichever crates happen to be present and plugin order is not a contract a
/// host should have to reason about.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SectionOrder(pub i32);

/// Marks the node a stack builds its sections into. Put it on the pane, or on
/// any node that should hold a column of cards.
#[derive(Component, Debug, Clone, Copy, Default)]
#[require(Node, SectionStyle)]
pub struct SectionStack;

/// Marker on the card frame the stack spawns for a section, carrying the id it
/// was built for.
///
/// A host looks for the matching [`SectionBody`] rather than this; it exists so
/// the reconcile can tell its own frames from anything else parented into the
/// stack.
#[derive(Component, Debug, Clone)]
pub struct SectionFrame(pub SectionId);

/// The node a section's rows go into. **A host finds its own by [`SectionId`]
/// and calls [`add_row`].**
///
/// The same inversion [`Page`](crate::Page) and `tempera-settings` use: the
/// stack builds the container, the owning crate fills it, and the stack never
/// learns what went in.
#[derive(Component, Debug, Clone)]
pub struct SectionBody(pub SectionId);

/// Marker on a row. The stack owns these; a section never gets one.
#[derive(Component, Debug, Clone, Copy)]
pub struct SectionRow;

/// Marker on the clipped cell a section's content lives in. The stack owns
/// these too.
#[derive(Component, Debug, Clone, Copy)]
pub struct SectionCell;

/// Per-stack geometry. On the stack rather than in a resource, so an inspector
/// and a browser can size their rows differently.
#[derive(Component, Debug, Clone, Copy)]
pub struct SectionStyle {
    /// Width reserved for the label column of a labelled row.
    pub label_width: f32,
    /// Height of a [`RowHeight::Control`] row's control.
    pub control: ControlSize,
    /// Gap between rows.
    pub row_gap: Step,
    /// Padding inside a card body.
    pub body_padding: Step,
}

impl Default for SectionStyle {
    fn default() -> Self {
        Self {
            label_width: 88.0,
            control: ControlSize::Sm,
            row_gap: Step::new(1),
            body_padding: Step::new(2),
        }
    }
}

/// How tall a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowHeight {
    /// A declared height from the theme's control sizes.
    ///
    /// The row cannot be grown by what goes in it, so the rows below never
    /// shift. This is the containment guarantee, and it is the default.
    #[default]
    Control,
    /// Sized by its content.
    ///
    /// **A deliberate opt-out.** A modulation matrix or a list grows with what
    /// it holds and cannot state a height up front. Such a row gives up only
    /// its *own* height: its cell still declares a width and still clips, so
    /// content cannot spill sideways or displace a sibling horizontally.
    Content,
}

/// What a row looks like. Build with [`Row::labelled`] or [`Row::full_width`].
#[derive(Debug, Clone, Copy)]
pub struct Row {
    label: Option<&'static str>,
    height: RowHeight,
}

impl Row {
    /// A label on the left, the caller's content in a fixed cell on the right.
    pub fn labelled(label: &'static str) -> Self {
        Self {
            label: Some(label),
            height: RowHeight::Control,
        }
    }

    /// No label; the cell spans the row.
    ///
    /// For content that is not one control — a matrix, a list, a preview. One
    /// constructor with the label optional rather than two functions, so there
    /// is a single path by which content enters a section and a single place
    /// the containment is applied.
    pub fn full_width() -> Self {
        Self {
            label: None,
            height: RowHeight::Control,
        }
    }

    /// Let the row size to its content — see [`RowHeight::Content`].
    #[must_use]
    pub fn grows(mut self) -> Self {
        self.height = RowHeight::Content;
        self
    }
}

/// Add a row to `body` and return **the entity to fill**.
///
/// The row and its cell are not returned and cannot be reached from what is:
/// that is what stops a section altering the panel's geometry. Whatever the
/// caller spawns under the returned entity is its own business.
pub fn add_row(
    commands: &mut Commands,
    metrics: &Metrics,
    palette: &ColorPalette,
    style: &SectionStyle,
    body: Entity,
    row: Row,
) -> Entity {
    let row_node = match row.height {
        // Declared: cannot be grown from inside. See the module docs.
        RowHeight::Control => Node {
            width: Val::Percent(100.0),
            height: Val::Px(metrics.control(style.control).get()),
            align_items: AlignItems::Center,
            ..default()
        },
        RowHeight::Content => Node {
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            ..default()
        },
    };

    let row_entity = commands.spawn((SectionRow, row_node, ChildOf(body))).id();

    if let Some(label) = row.label {
        commands.spawn((
            Text::new(label),
            TextColor(palette.muted_foreground),
            Node {
                width: Val::Px(style.label_width),
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
            ChildOf(row_entity),
        ));
    }

    commands
        .spawn((
            SectionCell,
            Node {
                // Both are obligations, not preferences:
                //   a width the *row* decides, or content inflates the cell;
                //   `clip()`, or content paints over its neighbours.
                // Neither is implied by the other — see the layout tests.
                //
                // `flex_basis: 0` with `flex_grow: 1` is what makes the width
                // come from the row rather than from what the section put in:
                // the cell starts at nothing and takes exactly the space left
                // over, so a 9999px child cannot bid for more. Setting
                // `width` instead would be sizing *around* the content, which
                // is the failure mode the layout tests found.
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            ChildOf(row_entity),
        ))
        .id()
}

/// Build a card frame per declared section, drop frames whose section is gone,
/// and keep them ordered.
///
/// One system for all three because they share the sorted list, and because
/// splitting them would let a frame appear at the wrong end and jump.
///
/// Runs as an exclusive system: a section's [`When`](tempera_input::When) is a
/// `System`, and evaluating one needs `&mut World`.
pub(crate) fn reconcile_sections(world: &mut World) {
    let stacks: Vec<(Entity, SectionStyle)> = world
        .query_filtered::<(Entity, &SectionStyle), With<SectionStack>>()
        .iter(world)
        .map(|(e, s)| (e, *s))
        .collect();

    for (stack, _style) in stacks {
        let declared: Vec<Entity> = world
            .get::<Children>(stack)
            .map(|kids| kids.iter().collect())
            .unwrap_or_default();

        // Sections are declared as children of the stack; frames are spawned
        // as children too, so tell them apart before doing anything else.
        let mut wanted: Vec<(i32, SectionId, Entity)> = declared
            .iter()
            .filter(|e| world.get::<Section>(**e).is_some())
            .filter_map(|e| {
                let id = world.get::<SectionId>(*e)?.clone();
                let order = world.get::<SectionOrder>(*e).copied().unwrap_or_default();
                Some((order.0, id, *e))
            })
            .collect();
        wanted.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        // A gate that does not pass means no frame at all, rather than a frame
        // that is present and empty.
        wanted.retain(|(_, _, entity)| tempera_input::condition::passes(world, *entity));

        let existing: Vec<(Entity, SectionId)> = declared
            .iter()
            .filter_map(|e| world.get::<SectionFrame>(*e).map(|f| (*e, f.0.clone())))
            .collect();

        let same = existing.len() == wanted.len()
            && existing
                .iter()
                .zip(&wanted)
                .all(|((_, have), (_, want, _))| have == want);
        if same {
            continue;
        }

        for (frame, _) in existing {
            world.entity_mut(frame).despawn();
        }
        for (_, id, _) in &wanted {
            spawn_frame(world, stack, id.clone());
        }
    }
}

/// One card: a heading and an empty body for its owner to fill.
fn spawn_frame(world: &mut World, stack: Entity, id: SectionId) {
    let style = world
        .get::<SectionStyle>(stack)
        .copied()
        .unwrap_or_default();
    let label = world
        .query_filtered::<(&SectionId, &SectionLabel), With<Section>>()
        .iter(world)
        .find(|(sid, _)| **sid == id)
        .map(|(_, l)| l.0)
        .unwrap_or("");

    let frame = world
        .spawn((
            SectionFrame(id.clone()),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ChildOf(stack),
        ))
        .id();

    if !label.is_empty() {
        world.spawn((
            Text::new(label),
            Node::default(),
            bevy::picking::Pickable::IGNORE,
            ChildOf(frame),
        ));
    }

    world.spawn((
        SectionBody(id),
        StyledNode::new()
            .padding_x(style.body_padding)
            .row_gap(style.row_gap),
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        ChildOf(frame),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempera_input::When;

    #[derive(Resource, Default)]
    struct Symbolic(bool);

    fn is_symbolic(s: Res<Symbolic>) -> bool {
        s.0
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Symbolic>()
            .add_systems(Update, reconcile_sections);
        app
    }

    fn stack_with(app: &mut App, sections: &[(&str, i32)]) -> Entity {
        let stack = app.world_mut().spawn(SectionStack).id();
        for (id, order) in sections {
            app.world_mut().spawn((
                Section,
                SectionId::from(*id),
                SectionOrder(*order),
                ChildOf(stack),
            ));
        }
        app.update();
        stack
    }

    fn frame_ids(app: &mut App, stack: Entity) -> Vec<String> {
        let kids: Vec<Entity> = app
            .world()
            .get::<Children>(stack)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        kids.iter()
            .filter_map(|k| app.world().get::<SectionFrame>(*k))
            .map(|f| f.0.0.clone())
            .collect()
    }

    #[test]
    fn a_frame_appears_for_every_declared_section() {
        let mut app = app();
        let stack = stack_with(&mut app, &[("transport", 10), ("effects", 20)]);
        assert_eq!(frame_ids(&mut app, stack), ["transport", "effects"]);
    }

    #[test]
    fn frames_follow_order_not_declaration() {
        // Sections are declared by whichever crates are present, so plugin
        // order must not decide what the panel looks like.
        let mut app = app();
        let stack = stack_with(&mut app, &[("last", 30), ("first", 10), ("mid", 20)]);
        assert_eq!(frame_ids(&mut app, stack), ["first", "mid", "last"]);
    }

    #[test]
    fn a_section_declared_later_still_gets_a_frame() {
        // The failure the implementation this replaces had: it returned early
        // once any section existed, so a crate registering after startup was
        // unreachable.
        let mut app = app();
        let stack = stack_with(&mut app, &[("transport", 10)]);
        assert_eq!(frame_ids(&mut app, stack).len(), 1);

        app.world_mut().spawn((
            Section,
            SectionId::from("effects"),
            SectionOrder(20),
            ChildOf(stack),
        ));
        app.update();

        assert_eq!(frame_ids(&mut app, stack), ["transport", "effects"]);
    }

    #[test]
    fn a_gated_section_is_absent_rather_than_empty() {
        // Not hidden, not present-and-blank: no frame at all. A panel showing
        // ten headings of which seven say nothing is the thing this replaces.
        let mut app = app();
        let stack = app.world_mut().spawn(SectionStack).id();
        app.world_mut().spawn((
            Section,
            SectionId::from("transport"),
            SectionOrder(10),
            ChildOf(stack),
        ));
        app.world_mut().spawn((
            Section,
            SectionId::from("note"),
            SectionOrder(20),
            When::new(is_symbolic),
            ChildOf(stack),
        ));
        app.update();

        assert_eq!(frame_ids(&mut app, stack), ["transport"]);
    }

    #[test]
    fn a_gate_that_opens_brings_its_section_back() {
        let mut app = app();
        let stack = app.world_mut().spawn(SectionStack).id();
        app.world_mut().spawn((
            Section,
            SectionId::from("note"),
            SectionOrder(10),
            When::new(is_symbolic),
            ChildOf(stack),
        ));
        app.update();
        assert!(frame_ids(&mut app, stack).is_empty());

        app.world_mut().resource_mut::<Symbolic>().0 = true;
        app.update();

        assert_eq!(frame_ids(&mut app, stack), ["note"]);
    }

    #[test]
    fn every_frame_carries_a_body_for_its_owner_to_fill() {
        // The inversion: the stack builds the container and never learns what
        // goes in it.
        let mut app = app();
        let stack = stack_with(&mut app, &[("transport", 10)]);

        let bodies: Vec<String> = app
            .world_mut()
            .query::<&SectionBody>()
            .iter(app.world())
            .map(|b| b.0.0.clone())
            .collect();
        assert_eq!(bodies, ["transport"]);
        let _ = stack;
    }

    #[test]
    fn a_settled_stack_is_not_rebuilt() {
        // Re-spawning frames would drop whatever their owners had put inside.
        let mut app = app();
        let stack = stack_with(&mut app, &[("a", 10), ("b", 20)]);
        let before: Vec<Entity> = app.world().get::<Children>(stack).unwrap().iter().collect();

        // Something changes elsewhere; this stack must not churn.
        app.world_mut().spawn(SectionStack);
        app.update();

        let after: Vec<Entity> = app.world().get::<Children>(stack).unwrap().iter().collect();
        assert_eq!(before, after, "frames must survive an unrelated change");
    }
}
