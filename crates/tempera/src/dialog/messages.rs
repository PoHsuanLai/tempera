use bevy::prelude::*;

/// Fired when the user dismisses a dialog by clicking the backdrop,
/// clicking the close button, or pressing Escape while the dialog is
/// visible. The `dialog` field is the dialog root entity (the one
/// carrying the [`super::components::Dialog`] component).
///
/// The dialog itself does **not** auto-hide on dismissal — consumers
/// own the visibility state (and any associated app-state). React to
/// this message by flipping your own state resource / the dialog
/// root's `Visibility::Hidden`.
#[derive(Message, Debug, Clone, Copy)]
pub struct DialogDismissed {
    pub dialog: Entity,
}
