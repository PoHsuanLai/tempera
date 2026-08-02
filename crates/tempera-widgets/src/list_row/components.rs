use bevy::prelude::*;

/// Root marker on one row of an addressable list.
///
/// Behaviour-free apart from hover. What a click *means* is the caller's:
/// a row is an installed extension in one app, a keybinding in another, a
/// layer in a third.
#[derive(Component, Default, Debug)]
pub struct ListRow;

/// Stable identity for a row, so a rebuild can address one.
///
/// This is what separates a list row from a form row. A list is
/// *reconciled* — filtered, sorted, re-emitted when its source changes —
/// and every one of those operations needs to name a row without holding
/// its `Entity`, which a despawn-and-respawn invalidates.
///
/// Owned `String` rather than `&'static str`: the ids come from scans and
/// user data, not from source code.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ListRowId(pub String);

impl ListRowId {
    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for ListRowId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// The row's leading column — the flex column holding title, subtitle and
/// whatever else the caller adds.
///
/// Marked so a caller can parent extra content into it after the fact
/// without walking children and guessing.
#[derive(Component, Default, Debug)]
pub struct ListRowLead;

/// The row's trailing slot. **Parent your controls here.**
///
/// This is the load-bearing part of the widget. It holds *N* widgets, not
/// one: an extension row puts a trash button beside a switch, a keybinding
/// row puts a reset link beside a keycap chip. A fixed-width single-control
/// slot — which is what a settings row has — cannot express either, and
/// that is precisely why both were hand-rolled instead of reusing one.
///
/// Sized by `min_width` rather than `width`, so the trailing content grows
/// to fit itself and the leading column takes the rest.
#[derive(Component, Default, Debug)]
pub struct ListRowTrail;

/// Marker on the row's title `Text`.
#[derive(Component, Default, Debug)]
pub struct ListRowTitle;

/// Marker on the row's subtitle `Text` — a description, a category, a path.
#[derive(Component, Default, Debug)]
pub struct ListRowSubtitle;

/// Marker on a badge `Text` beside the title — a kind, a format, a state.
#[derive(Component, Default, Debug)]
pub struct ListRowBadge;

/// Marker on the muted text beside the title — a version, a count.
#[derive(Component, Default, Debug)]
pub struct ListRowMeta;
