//! Where a card's theme-derived geometry is applied.
//!
//! # Why this is a system and not part of spawning
//!
//! A spawn function reads the theme once and bakes the answer into a `Node`.
//! That is invisible until something changes the theme — then every widget
//! already on screen keeps the geometry it was born with, and only newly
//! spawned ones pick up the change. It is the same defect the colour systems
//! had before they moved to run conditions, one layer down.
//!
//! It also blocks scenes. A `bsn!` scene is built without world access, so a
//! scene that carried geometry would have to bake constants — reintroducing
//! exactly what the token work removed. With geometry applied by a system,
//! a scene names only structure and the theme fills in the rest.
//!
//! # Field-granular, and compared before writing
//!
//! The system writes *individual fields*, never a whole `Node`. Three
//! widgets — `slider`, `switch`, `tabs` — animate `left`/`width` every
//! frame, so a whole-`Node` write would fight the animation and the thumb
//! would stutter or snap back.
//!
//! Every write compares first, because `bevy_ui` gates its taffy upload on
//! `Ref<Node>::is_changed()` and a `DerefMut` sets that flag whether or not
//! the value moved. Writing unconditionally would re-upload the layout of
//! every card, every frame.

use bevy::prelude::*;

use super::components::{Card, CardBody, CardChevron, CardHeader};
use crate::theme::{ControlSize, Metrics, Step};

/// One part of a card, excluding the other three.
///
/// Bevy proves two `&mut Node` queries disjoint from their filters alone, so
/// each part must exclude *every* other marker rather than only the ones
/// that happen to collide today — a partial exclusion set panics at first
/// run. Spelled once as an alias because writing it four times inline is
/// what earns a `type_complexity` warning.
type OnlyPart<'w, 's, A, B, C, D> =
    Query<'w, 's, &'static mut Node, (With<A>, Without<B>, Without<C>, Without<D>)>;

/// Apply the theme's geometry to every card, header, chevron and body.
///
/// Gated on [`crate::theme::tokens_changed`] plus newly-spawned cards, so a
/// settled screen costs nothing.
pub(crate) fn apply_card_metrics(
    metrics: Metrics,
    mut cards: OnlyPart<Card, CardHeader, CardBody, CardChevron>,
    mut headers: OnlyPart<CardHeader, Card, CardBody, CardChevron>,
    mut bodies: OnlyPart<CardBody, Card, CardHeader, CardChevron>,
    mut chevrons: OnlyPart<CardChevron, Card, CardHeader, CardBody>,
) {
    let gutter = metrics.gap(Step::new(2)).get();
    let card_padding = UiRect::axes(Val::Px(gutter), metrics.gap(Step::BASE).into());
    let card_radius = BorderRadius::all(metrics.radius(Step::new(1)).into());
    let header_height: Val = metrics.control(ControlSize::Sm).into();
    let chevron_box = Val::Px(metrics.gap(Step::new(3)).get());
    let body_gap = Val::Px(gutter);

    for mut node in &mut cards {
        if node.padding != card_padding {
            node.padding = card_padding;
        }
        if node.border_radius != card_radius {
            node.border_radius = card_radius;
        }
    }
    for mut node in &mut headers {
        if node.height != header_height {
            node.height = header_height;
        }
    }
    for mut node in &mut chevrons {
        if node.width != chevron_box {
            node.width = chevron_box;
        }
        if node.height != chevron_box {
            node.height = chevron_box;
        }
    }
    for mut node in &mut bodies {
        if node.row_gap != body_gap {
            node.row_gap = body_gap;
        }
    }
}
