use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use super::components::{Dialog, DialogBackdrop, DialogCard, DialogTitle, DialogTitleBar};
use super::messages::DialogDismissed;
use crate::theme::ColorPalette;

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

/// Repaint every dialog surface from the current palette.
///
/// The dialog paints five things at spawn — the backdrop's scrim, the card's
/// fill and border, the title row's rule, and the title's text — and a value
/// written once at spawn cannot follow a theme change. A dialog opened before
/// the user switched theme stayed in the old one for the life of the dialog,
/// which for a settings dialog is the *entire* time they are choosing a theme.
///
/// Split across four queries rather than one hierarchy walk because each
/// surface has its own marker and its own target component; walking from the
/// root would have to re-derive which node is which, and that is what the
/// markers already answer.
///
/// Each write compares first, so the frames where nothing moved cost a handful
/// of `Color` comparisons and mark nothing changed — the same bargain every
/// other repaint system in this crate makes, and the reason this can run
/// unfiltered under a run condition rather than on a `Changed` filter. A
/// palette swap makes every dialog stale at once, which is not a fact about
/// any single entity and so cannot be expressed as a query filter.
pub(crate) fn repaint_dialog_surfaces(
    palette: Res<ColorPalette>,
    mut backdrops: Query<&mut BackgroundColor, (With<DialogBackdrop>, Without<DialogCard>)>,
    mut cards: Query<(&mut BackgroundColor, &mut BorderColor), With<DialogCard>>,
    mut title_bars: Query<&mut BorderColor, (With<DialogTitleBar>, Without<DialogCard>)>,
    mut titles: Query<&mut TextColor, With<DialogTitle>>,
) {
    for mut bg in &mut backdrops {
        if bg.0 != palette.scrim {
            bg.0 = palette.scrim;
        }
    }
    for (mut bg, mut border) in &mut cards {
        if bg.0 != palette.card {
            bg.0 = palette.card;
        }
        let want = BorderColor::all(palette.border);
        if *border != want {
            *border = want;
        }
    }
    for mut border in &mut title_bars {
        let want = BorderColor::all(palette.border);
        if *border != want {
            *border = want;
        }
    }
    for mut color in &mut titles {
        if color.0 != palette.foreground {
            color.0 = palette.foreground;
        }
    }
}
