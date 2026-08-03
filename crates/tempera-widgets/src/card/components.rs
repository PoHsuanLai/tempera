//! Card markers and state.

use bevy::prelude::*;

/// A titled panel with a collapsible body.
#[derive(Component, Default, Debug)]
pub struct Card;

/// The clickable strip holding a card's title and chevron.
#[derive(Component, Default, Debug)]
pub struct CardHeader;

/// The part a collapse hides.
///
/// Its `Display` is the only thing collapsing touches, so a body keeps its
/// own layout — flex direction, gaps, children — across a collapse and
/// re-expand. Rebuilding it instead would lose scroll position and any
/// widget state inside.
#[derive(Component, Default, Debug)]
pub struct CardBody;

/// The chevron image, naming the card it belongs to.
///
/// The back-reference exists because a chevron is a *grandchild* — card →
/// header → chevron — and a paint system that walked down to find it needed
/// the header query, the children-of query and the chevron query all at
/// once, purely as navigation. Naming the card instead lets the system query
/// chevrons directly and ask one question about each.
#[derive(Component, Debug)]
pub struct CardChevron(pub Entity);

/// Present on a card whose body is showing; absent when collapsed.
///
/// A marker rather than a `CardCollapsed(bool)`, matching
/// [`crate::tree_row::TreeRowExpanded`] and Bevy's own `Checked`: the state
/// *is* the presence, so "every open card" is a query filter rather than an
/// iterate-and-test, and a caller cannot leave the flag disagreeing with the
/// art by writing the bool without running the system.
///
/// It also names the state positively. `CardCollapsed(false)` has to be read
/// twice to mean "open".
#[derive(Component, Default, Debug)]
pub struct CardExpanded;

/// Which way a card opens when it is spawned.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CardState {
    /// Body hidden.
    Collapsed,
    /// Body showing.
    #[default]
    Expanded,
}

impl CardState {
    /// Whether this state means the body is showing.
    #[must_use]
    #[inline]
    pub const fn is_expanded(self) -> bool {
        matches!(self, Self::Expanded)
    }
}
