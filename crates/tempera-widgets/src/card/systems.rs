//! Mirror [`CardExpanded`] onto the body's `Display` and the chevron's art.

use bevy::prelude::*;

use super::components::{Card, CardBody, CardChevron, CardExpanded};
use super::spawn::CardTokens;
use crate::theme::ColorPalette;

/// Show or hide each card's body to match [`CardExpanded`].
///
/// Runs unfiltered rather than on `Changed<CardExpanded>`, because
/// **`Changed` does not fire when a component is removed** — and collapsing
/// a card *is* a removal. A filtered version leaves the body visible after
/// the first collapse, which reads as the click being ignored. `tree_row`
/// learned the same thing; its test says so explicitly.
///
/// Affordable because the write compares first, so a settled card costs one
/// comparison and marks nothing dirty. That matters for `Node` especially:
/// `bevy_ui` gates its taffy upload on `Ref<Node>::is_changed()`, and a
/// `DerefMut` sets that flag whether or not the value moved.
pub(crate) fn apply_card_body(
    cards: Query<(Has<CardExpanded>, &Children), With<Card>>,
    mut bodies: Query<&mut Node, With<CardBody>>,
) {
    for (expanded, kids) in &cards {
        let want = if expanded {
            Display::Flex
        } else {
            Display::None
        };
        for child in kids.iter() {
            if let Ok(mut node) = bodies.get_mut(child)
                && node.display != want
            {
                node.display = want;
            }
        }
    }
}

/// Point each chevron the way its card is facing, and tint it.
///
/// Separate from [`apply_card_body`] because they answer different
/// questions — one is layout, one is art — and because a chevron names its
/// own card, so this needs no tree walking at all.
pub(crate) fn apply_card_chevron(
    tokens: Res<CardTokens>,
    palette: Res<ColorPalette>,
    open: Query<Has<CardExpanded>, With<Card>>,
    chevrons: Query<(Entity, &CardChevron)>,
    mut images: Query<&mut ImageNode>,
    mut commands: Commands,
) {
    for (entity, chevron) in &chevrons {
        let Ok(expanded) = open.get(chevron.0) else {
            continue;
        };
        let art = if expanded {
            tokens.chevron_expanded.as_ref()
        } else {
            tokens.chevron_collapsed.as_ref()
        };
        let Some(handle) = art else { continue };

        match images.get_mut(entity) {
            Ok(mut image) => {
                if image.image != *handle {
                    image.image = handle.clone();
                }
                if image.color != palette.muted_foreground {
                    image.color = palette.muted_foreground;
                }
            }
            // First frame the art is available: the slot was reserved at
            // spawn without an `ImageNode`, so the title does not shift
            // sideways when the handle lands.
            Err(_) => {
                commands
                    .entity(entity)
                    .insert(ImageNode::new(handle.clone()).with_color(palette.muted_foreground));
            }
        }
    }
}
