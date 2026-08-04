use bevy::prelude::*;

/// Marker on the dialog root entity. Toggle the entity's
/// [`Visibility`] to show / hide the dialog — the dialog tree (backdrop
/// + card) is parented to this entity, so hiding the root hides the
/// whole modal.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Dialog;

/// Marker on the full-screen translucent backdrop child. Clicking the
/// backdrop fires [`DialogDismissed`] for the parent [`Dialog`].
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogBackdrop;

/// Marker on the centered card child. User content is parented to the
/// returned content entity (see [`super::spawn::DialogParts::content`]),
/// which sits inside the card.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogCard;

/// Marker on the card's title row.
///
/// It exists so the row's bottom border can be repainted when the palette
/// changes. Without a marker there is no way to find the row again — it is
/// spawned as an anonymous child of the card — and its border would stay the
/// old theme's colour for the life of the dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogTitleBar;

/// Marker on the dialog's title text.
///
/// Same reason as [`DialogTitleBar`]: a `TextColor` written once at spawn
/// cannot follow a theme change, and the text node is otherwise indistinguishable
/// from any other `Text` in the tree.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogTitle;

/// Marker on the optional close-button entity in the card's title row.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogClose;

/// Marker on the content slot — the bevy_ui Node user code parents its
/// custom widgets into.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogContent;
