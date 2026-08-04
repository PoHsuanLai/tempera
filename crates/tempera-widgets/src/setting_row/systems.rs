//! Keeping a row's text in step with the palette.

use bevy::prelude::*;

use super::components::{SettingRowDescription, SettingRowLabel, SettingSectionLabel};
use crate::theme::ColorPalette;

/// What [`repaint_text`] reads per text node: the colour to write, and which
/// of the row's three kinds of label it is.
type RowText<'a> = (
    &'a mut TextColor,
    Option<&'a SettingRowLabel>,
    Option<&'a SettingRowDescription>,
    Option<&'a SettingSectionLabel>,
);

/// Repaint a row's label, its description, and any section heading.
///
/// The row is otherwise paint-free — it has no hover state and no fill of its
/// own — so this is the whole of its appearance that can go stale.
///
/// Why it was not obvious: two of the three colours are `muted_foreground`,
/// which is mid-grey in both palettes and legible either way. Only the label
/// takes `foreground`, which inverts — so in a light theme the label vanished
/// while the description above it stayed put, and the widget looked half
/// broken rather than unthemed.
pub(crate) fn repaint_text(
    palette: Res<ColorPalette>,
    // Three separate queries would need `Without` filters against each other
    // to prove disjointness to the borrow checker, which reads as if the three
    // could overlap. They cannot — every one of these markers sits on its own
    // `Text` node — so one query over the union says that plainly and lets the
    // match express which colour each kind takes.
    mut texts: Query<RowText>,
) {
    for (mut color, label, description, section) in &mut texts {
        let want = match (label, description, section) {
            (Some(_), _, _) => palette.foreground,
            (_, Some(_), _) | (_, _, Some(_)) => palette.muted_foreground,
            // Not one of ours — a caller's own text parented into the row.
            _ => continue,
        };
        if color.0 != want {
            color.0 = want;
        }
    }
}
