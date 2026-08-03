//! Spawn helper, tokens, and the `CardStyle` bundle.

use bevy::ecs::system::SystemParam;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;

use super::components::{Card, CardBody, CardChevron, CardExpanded, CardHeader, CardState};
use crate::theme::{ColorPalette, ControlSize, FontHandle, Step, StyledNode, Typography};

/// Sizing and art for [`Card`]s.
#[derive(Resource, Clone, Debug, Default)]
pub struct CardTokens {
    /// Chevron pointing down — the body is showing.
    pub chevron_expanded: Option<Handle<Image>>,
    /// Chevron pointing right — the body is hidden.
    pub chevron_collapsed: Option<Handle<Image>>,
}

/// The slice of theme tokens a card reads *at spawn*.
///
/// Colour and type only — geometry is applied by `apply_card_metrics` and
/// deliberately absent here. If a `Metrics` reappears in this bundle, some
/// size has drifted back into the spawn path.
#[derive(SystemParam)]
pub struct CardStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
}

/// The two entities a caller needs after spawning.
#[derive(Clone, Copy, Debug)]
pub struct CardParts {
    /// The card root — carries [`Card`] and, when open, [`CardExpanded`].
    /// Attach observers here.
    pub card: Entity,
    /// The body. Parent content into this.
    pub body: Entity,
}

/// Spawn a titled card with a collapsible body.
///
/// The header toggles [`CardExpanded`] on click; a paint system mirrors that
/// onto the body's `Display` and swaps the chevron. Content goes into
/// [`CardParts::body`].
///
/// ```ignore
/// let parts = spawn_card(&mut commands, &style, parent, "Transport", CardState::Expanded);
/// commands.entity(parts.body).with_children(|b| { /* … */ });
/// ```
pub fn spawn_card(
    commands: &mut Commands,
    style: &CardStyle,
    parent: Entity,
    title: impl Into<String>,
    state: CardState,
) -> CardParts {
    // `Node` carries structure — flex direction, 100% widths — and
    // `StyledNode` *declares* the theme-derived half. One system in the
    // theme crate resolves every declaration, so a card already on screen
    // follows a base or density change instead of keeping the geometry it
    // was born with, and this widget writes no geometry system of its own.
    let card = commands
        .spawn((
            Card,
            StyledNode::new()
                .padding_x(Step::new(2))
                .padding_y(Step::BASE)
                .radius(Step::new(1)),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // `card` rather than `background`: a card sits *on* the surface
            // behind it, and reading the same token as that surface would
            // make its edges invisible.
            BackgroundColor(style.palette.card),
            ChildOf(parent),
            Name::new("tempera::card"),
        ))
        .id();
    if state.is_expanded() {
        commands.entity(card).insert(CardExpanded);
    }

    let header = commands
        .spawn((
            CardHeader,
            StyledNode::new().height(ControlSize::Sm),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            crate::cursor::HoverCursor::default(),
            ChildOf(card),
        ))
        .observe(
            move |mut on: On<Pointer<Click>>,
                  mut commands: Commands,
                  open: Query<&CardExpanded>| {
                if open.get(card).is_ok() {
                    commands.entity(card).remove::<CardExpanded>();
                } else {
                    commands.entity(card).insert(CardExpanded);
                }
                on.propagate(false);
            },
        )
        .id();

    commands.spawn((
        Text::new(title.into()),
        style.font.text_font_bold(style.typography.sm),
        TextColor(style.palette.foreground),
        bevy::picking::Pickable::IGNORE,
        ChildOf(header),
    ));

    // Spawned without an `ImageNode`: the art is a token the host supplies,
    // and it may not be loaded yet. Reserving the box here keeps the title
    // from shifting when the handle arrives — the same reason `tree_row`
    // reserves its chevron slot.
    commands.spawn((
        CardChevron(card),
        StyledNode::new().square(Step::new(3)),
        Node::default(),
        bevy::picking::Pickable::IGNORE,
        ChildOf(header),
    ));

    let body = commands
        .spawn((
            CardBody,
            StyledNode::new().row_gap(Step::new(2)),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                display: if state.is_expanded() {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
            ChildOf(card),
        ))
        .id();

    CardParts { card, body }
}
