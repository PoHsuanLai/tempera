//! Mirror [`CardExpanded`] onto the body's `Display` and the chevron's art.

use bevy::prelude::*;
use bevy_resvg::prelude::{SvgColor, UiSvg};

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
/// Both the art and the tint are *declared* here — `UiSvg` for which glyph,
/// `SvgColor` for what colour — and `bevy_resvg` applies them to the
/// `ImageNode` it owns. The slot is still reserved at spawn without art, so
/// the title does not shift sideways when the handle lands.
pub(crate) fn apply_card_chevron(
    tokens: Res<CardTokens>,
    palette: Res<ColorPalette>,
    open: Query<Has<CardExpanded>, With<Card>>,
    chevrons: Query<(Entity, &CardChevron, Option<&UiSvg>, Option<&SvgColor>)>,
    mut commands: Commands,
) {
    for (entity, chevron, current_art, current_tint) in &chevrons {
        let Ok(expanded) = open.get(chevron.0) else {
            continue;
        };
        let art = if expanded {
            tokens.chevron_expanded.as_ref()
        } else {
            tokens.chevron_collapsed.as_ref()
        };
        let Some(handle) = art else { continue };

        // Write only on a change: `bevy_resvg` re-renders on `Changed<UiSvg>`
        // and re-tints on `Changed<SvgColor>`, so an unconditional insert
        // would rasterise every chevron every frame.
        if current_art.is_none_or(|c| c.0 != *handle) {
            commands.entity(entity).insert(UiSvg(handle.clone()));
        }
        if current_tint.is_none_or(|c| c.0 != palette.muted_foreground) {
            commands
                .entity(entity)
                .insert(SvgColor(palette.muted_foreground));
        }
    }
}
