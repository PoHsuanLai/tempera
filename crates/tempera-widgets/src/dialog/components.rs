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

/// Marker on the optional close-button entity in the card's title row.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogClose;

/// Marker on the content slot — the bevy_ui Node user code parents its
/// custom widgets into.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogContent;
