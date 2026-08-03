//! Spawn helper, tokens, and the `CardStyle` bundle.

use bevy::ecs::system::SystemParam;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;

use super::components::{Card, CardBody, CardChevron, CardExpanded, CardHeader, CardState};
use crate::theme::{ColorPalette, ControlSize, FontHandle, Metrics, Step, Typography};

/// Sizing and art for [`Card`]s.
#[derive(Resource, Clone, Debug, Default)]
pub struct CardTokens {
    /// Chevron pointing down — the body is showing.
    pub chevron_expanded: Option<Handle<Image>>,
    /// Chevron pointing right — the body is hidden.
    pub chevron_collapsed: Option<Handle<Image>>,
}

/// The slice of theme tokens a card reads.
#[derive(SystemParam)]
pub struct CardStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub metrics: Metrics<'w>,
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
    let gutter = style.metrics.gap(Step::new(2)).get();
    let card = commands
        .spawn((
            Card,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(gutter), style.metrics.gap(Step::BASE).into()),
                border_radius: BorderRadius::all(style.metrics.radius(Step::new(1)).into()),
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
            Node {
                width: Val::Percent(100.0),
                height: style.metrics.control(ControlSize::Sm).into(),
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
    let chevron_box = style.metrics.gap(Step::new(3)).get();
    commands.spawn((
        CardChevron(card),
        Node {
            width: Val::Px(chevron_box),
            height: Val::Px(chevron_box),
            ..default()
        },
        bevy::picking::Pickable::IGNORE,
        ChildOf(header),
    ));

    let body = commands
        .spawn((
            CardBody,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(gutter),
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
