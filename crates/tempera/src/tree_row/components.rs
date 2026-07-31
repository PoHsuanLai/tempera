use bevy::prelude::*;

/// Root marker on one row of an indented list.
///
/// Behaviour-free apart from hover: the widget paints, and *what a click
/// means* is the caller's, because a row is a file in one app, a layer in
/// another and a MIDI device in a third.
#[derive(Component, Default, Debug)]
pub struct TreeRow;

/// Marker on the row's label `Text` child.
///
/// Exists so the hover repaint can find the label directly instead of
/// walking children and guessing which `Text` is the label rather than
/// the suffix.
#[derive(Component, Default, Debug)]
pub struct TreeRowLabel;

/// Marker on the trailing text child — a file extension, a plugin format,
/// a count. Rendered smaller and dimmer than the label.
#[derive(Component, Default, Debug)]
pub struct TreeRowSuffix;

/// Marker on the disclosure chevron's `ImageNode`.
///
/// Present only on rows spawned with [`TreeRowSpec::chevron`]. Flip
/// [`TreeRowExpanded`] on the row and the paint system swaps the image.
#[derive(Component, Default, Debug)]
pub struct TreeRowChevron;

/// Present on a row whose chevron points open; absent when it points
/// closed.
///
/// A marker rather than a `bool` field, matching how Bevy's own `Checked`
/// models a checkbox: the state is the presence, so a query for open rows
/// is a filter rather than a scan.
#[derive(Component, Default, Debug)]
pub struct TreeRowExpanded;

/// Marker on a row spawned with [`TreeRowSpec::header`] — rendered at the
/// brighter foreground token, for a row that names a group rather than
/// an item inside one.
#[derive(Component, Default, Debug)]
pub struct TreeRowHeader;

/// Which way a row's chevron points when it is spawned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChevronState {
    /// Pointing right — the group is collapsed.
    #[default]
    Collapsed,
    /// Pointing down — the group is expanded.
    Expanded,
}

impl ChevronState {
    /// Whether this state means expanded.
    pub fn is_expanded(self) -> bool {
        matches!(self, Self::Expanded)
    }
}
