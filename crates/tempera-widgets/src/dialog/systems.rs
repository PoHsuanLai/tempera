use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use super::components::Dialog;
use super::messages::DialogDismissed;

/// Z-order for dialog modals. Above tooltips and menus, below toasts
/// (toasts use `Z_TOAST = 3000`).
pub(crate) const Z_DIALOG: i32 = 2500;

/// Press Escape → fire [`DialogDismissed`] for every currently-visible
/// dialog. Consumers decide whether to hide.
///
/// Reads `KeyboardInput` messages directly (not `ButtonInput<KeyCode>`)
/// so that key-edge events are caught even when other systems consume
/// the focused-input state.
pub(crate) fn dismiss_on_escape(
    mut keys: MessageReader<KeyboardInput>,
    dialogs: Query<(Entity, &Visibility), With<Dialog>>,
    mut writer: MessageWriter<DialogDismissed>,
) {
    let escape_pressed = keys
        .read()
        .any(|k| k.state == ButtonState::Pressed && k.key_code == KeyCode::Escape);
    if !escape_pressed {
        return;
    }
    for (entity, visibility) in &dialogs {
        if *visibility != Visibility::Hidden {
            writer.write(DialogDismissed { dialog: entity });
        }
    }
}
