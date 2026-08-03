use bevy::prelude::*;

use super::components::ListRow;
use crate::theme::ColorPalette;

/// What [`repaint_rows`] reads and writes per row.
type RowPaint<'a> = (&'a Interaction, &'a mut BackgroundColor);

/// Paint the hover fill.
///
/// Driven by `Interaction` rather than by `Pointer<Over>`/`Pointer<Out>`
/// observers, which is what the two hand-rolled implementations this
/// replaces both used. The observer form has a bug that a reconciled list
/// hits constantly: a list that despawns and respawns its rows under a
/// stationary pointer never delivers `Out` to the despawned row nor `Over`
/// to the new one, so the tint is simply wrong until the pointer moves.
/// Reading `Interaction` each frame cannot desynchronise that way.
///
/// The query carries no `Changed<Interaction>` filter, because a palette
/// swap makes every row stale at once and no *per-entity* filter can say
/// so. Staleness is decided by the run condition instead
/// (`Changed<Interaction>`-shaped work still only happens when something
/// moved), and the write below is compared first, so an unchanged row
/// costs one `Color` comparison and marks nothing.
pub(crate) fn repaint_rows(palette: Res<ColorPalette>, rows: Query<RowPaint, With<ListRow>>) {
    for (interaction, mut bg) in rows {
        let hovered = !matches!(interaction, Interaction::None);
        let want = if hovered { palette.muted } else { Color::NONE };
        if bg.0 != want {
            bg.0 = want;
        }
    }
}
