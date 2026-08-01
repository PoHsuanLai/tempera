use bevy::prelude::*;

use super::components::ListRow;
use crate::theme::ColorPalette;

/// What [`repaint_rows`] reads and writes per row.
type RowPaint<'a> = (&'a Interaction, &'a mut BackgroundColor);

/// Rows whose tint may be stale: the pointer moved, or the row is new.
type RepaintedRows = Or<(Changed<Interaction>, Added<ListRow>)>;

/// Paint the hover fill.
///
/// Driven by `Interaction` rather than by `Pointer<Over>`/`Pointer<Out>`
/// observers, which is what the two hand-rolled implementations this
/// replaces both used. The observer form has a bug that a reconciled list
/// hits constantly: a list that despawns and respawns its rows under a
/// stationary pointer never delivers `Out` to the despawned row nor `Over`
/// to the new one, so the tint is simply wrong until the pointer moves.
/// Reading `Interaction` each frame cannot desynchronise that way.
pub(crate) fn repaint_rows(palette: Res<ColorPalette>, rows: Query<RowPaint, RepaintedRows>) {
    for (interaction, mut bg) in rows {
        let hovered = !matches!(interaction, Interaction::None);
        bg.0 = if hovered { palette.muted } else { Color::NONE };
    }
}
