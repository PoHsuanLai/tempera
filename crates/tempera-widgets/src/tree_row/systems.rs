use bevy::prelude::*;

use super::components::{
    ChevronState, TreeRow, TreeRowChevron, TreeRowExpanded, TreeRowHeader, TreeRowLabel,
};
use super::spawn::{TreeRowTokens, chevron_handle};
use crate::theme::ColorPalette;

/// What [`repaint_rows`] reads and writes per row.
type RowPaint<'a> = (
    &'a Interaction,
    &'a Children,
    Has<TreeRowHeader>,
    &'a mut BackgroundColor,
);

/// A row's children and whether it is expanded.
type ChevronRow<'a> = (&'a Children, Has<TreeRowExpanded>);

/// Rows whose chevron may be stale.
type RepaintedChevrons = Or<(Changed<TreeRowExpanded>, Added<TreeRow>)>;

/// A chevron node, and its art if it has any yet.
type ChevronArt<'a> = (Entity, Option<&'a mut ImageNode>);

/// Paint the hover fill and lift the label toward the foreground.
///
/// Driven by `Interaction` rather than pointer observers so a row the
/// cursor never leaves — because it was despawned and respawned under a
/// stationary pointer, which a rebuilt list does constantly — still ends
/// up with the right tint.
/// The query is unfiltered and the writes are compared: a palette swap
/// makes every row stale at once, which no per-entity filter can say. See
/// [`crate::theme::repaint_needed`].
pub(crate) fn repaint_rows(
    palette: Res<ColorPalette>,
    rows: Query<RowPaint, With<TreeRow>>,
    mut labels: Query<&mut TextColor, With<TreeRowLabel>>,
) {
    for (interaction, children, is_header, mut bg) in rows {
        let hovered = !matches!(interaction, Interaction::None);
        let want = if hovered { palette.muted } else { Color::NONE };
        if bg.0 != want {
            bg.0 = want;
        }

        let label_color = if hovered || is_header {
            palette.foreground
        } else {
            palette.muted_foreground
        };
        for child in children.iter() {
            if let Ok(mut color) = labels.get_mut(child)
                && color.0 != label_color
            {
                color.0 = label_color;
            }
        }
    }
}

/// Swap each chevron's art to match its row's [`TreeRowExpanded`].
///
/// Runs on change only. A row whose chevron art has not loaded yet keeps
/// its reserved slot and picks the image up on a later frame.
pub(crate) fn repaint_chevrons(
    tokens: Res<TreeRowTokens>,
    palette: Res<ColorPalette>,
    changed: Query<ChevronRow, RepaintedChevrons>,
    mut removed: RemovedComponents<TreeRowExpanded>,
    all_rows: Query<ChevronRow>,
    mut chevrons: Query<ChevronArt, With<TreeRowChevron>>,
    mut commands: Commands,
) {
    let mut paint = |children: &Children, expanded: bool| {
        let state = if expanded {
            ChevronState::Expanded
        } else {
            ChevronState::Collapsed
        };
        let Some(handle) = chevron_handle(&tokens, state) else {
            return;
        };
        for child in children.iter() {
            let Ok((entity, image)) = chevrons.get_mut(child) else {
                continue;
            };
            match image {
                Some(mut image) => {
                    image.image = handle.clone();
                    image.color = palette.muted_foreground;
                }
                // The slot was reserved before the art existed; fill it in.
                None => {
                    commands.entity(entity).insert(
                        ImageNode::new(handle.clone()).with_color(palette.muted_foreground),
                    );
                }
            }
        }
    };

    for (children, expanded) in &changed {
        paint(children, expanded);
    }
    // `Changed` does not fire for a *removed* marker, so collapsing a row
    // would otherwise leave its chevron pointing open.
    for entity in removed.read() {
        if let Ok((children, expanded)) = all_rows.get(entity) {
            paint(children, expanded);
        }
    }
}
