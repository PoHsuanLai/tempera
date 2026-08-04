//! Recording a chord from the keyboard — the "press a shortcut" flow.
//!
//! [`Chord::Custom`] existed to receive this from the day it was written; its
//! doc comment names this recorder. Everything downstream of it already
//! shipped — [`rebind`](crate::rebind) writes the component *and* persists,
//! and [`KbdChord`](crate::KbdChord) renders the result — so this is the one
//! missing link between a keypress and a saved binding.
//!
//! # Who owns what
//!
//! [`ChordCapture`] is a resource, and it is the *only* state here: absent (or
//! `None`) means nobody is recording. A host starts a capture by writing it and
//! learns the outcome from [`ChordCaptured`]. Nothing in this module knows what
//! the chord is *for* — pairing it with a command is the caller's job, which is
//! what keeps a keybindings tab, a macro editor and a per-tool binder from
//! needing three versions of this.
//!
//! # When a chord is finished
//!
//! On the **release** of the first non-modifier key, not on press.
//!
//! Pressing `Cmd+Shift+K` means holding three keys, and at the instant `K` goes
//! down the user may still be reaching for a fourth. Capturing on press would
//! also make `Shift` alone — struck on the way to `Shift+A` — look like a
//! complete chord for one frame. Waiting for the release of the *terminating*
//! key is what makes "the modifiers I was holding" a well-defined set.
//!
//! Modifier-only chords are therefore unrepresentable, which is deliberate: a
//! binding on `Shift` alone would fire while typing any capital letter.

use bevy::input::ButtonInput;
use bevy::prelude::*;

use crate::chord::Chord;

/// An in-progress chord recording.
///
/// Presence *is* the recording state — a host inserts this to start listening
/// and the capture system removes it when a chord lands or the user cancels.
/// There is no `active: bool`, because a flag and a resource would be two
/// owners of one fact.
#[derive(Resource, Debug, Default, Clone)]
pub struct ChordCapture {
    /// Modifiers held during this recording, accumulated across frames.
    ///
    /// Accumulated rather than sampled at the terminating release, because a
    /// user often lifts a modifier and the key together and the exact frame
    /// ordering is not something a recorder should depend on.
    held_modifiers: Vec<KeyCode>,
}

/// The outcome of a recording.
///
/// A message rather than a component on some entity, because the thing that
/// *started* the capture is the thing that wants the answer, and it may be a
/// settings row, a command palette or a test.
#[derive(Message, Debug, Clone)]
pub enum ChordCaptured {
    /// The user pressed and released a usable chord.
    Chord(Chord),
    /// The user pressed Escape. Distinct from `Chord` because a caller almost
    /// always wants to leave the existing binding alone rather than clear it,
    /// and an `Option<Chord>` would make "cancelled" and "unbound" the same
    /// value.
    Cancelled,
}

/// Every key this module treats as a modifier rather than a chord terminator.
///
/// Both sides of each pair: a user pressing the right-hand Shift means Shift.
/// [`Chord::keys`] normalises to the left-hand variant when it *builds* a
/// chord, but a recorder has to accept what the keyboard actually reports.
const MODIFIERS: &[KeyCode] = &[
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::AltLeft,
    KeyCode::AltRight,
    KeyCode::SuperLeft,
    KeyCode::SuperRight,
];

fn is_modifier(key: KeyCode) -> bool {
    MODIFIERS.contains(&key)
}

/// Listen for a chord while [`ChordCapture`] is present.
///
/// Runs only when the resource exists, so an app that never records pays a
/// resource-existence check and nothing else.
pub fn capture_chord(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut capture: ResMut<ChordCapture>,
    mut out: MessageWriter<ChordCaptured>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<ChordCapture>();
        out.write(ChordCaptured::Cancelled);
        return;
    }

    // Accumulate modifiers as they go down. Recorded on press rather than read
    // at release, so a user who lifts Shift a frame before the letter still
    // gets Shift in the chord.
    for key in keys.get_just_pressed() {
        if is_modifier(*key) && !capture.held_modifiers.contains(key) {
            capture.held_modifiers.push(*key);
        }
    }

    // The terminating key is the first non-modifier to come *up*. Release
    // rather than press: at the instant a key goes down the user may still be
    // adding to the chord, and a modifier struck on the way to a longer chord
    // would otherwise register as a complete one.
    let Some(terminator) = keys
        .get_just_released()
        .find(|key| !is_modifier(**key))
        .copied()
    else {
        return;
    };

    let mut chord_keys = capture.held_modifiers.clone();
    chord_keys.push(terminator);

    commands.remove_resource::<ChordCapture>();
    out.write(ChordCaptured::Chord(from_keys(&chord_keys)));
}

/// Build the narrowest [`Chord`] variant that describes these keys.
///
/// Narrowest rather than always `Custom`, so a captured chord is
/// indistinguishable from one written by hand — `Chord::Cmd(KeyCode::KeyS)`
/// either way. That matters for equality, for `matches!` in host code, and for
/// anything that reasons about a binding by variant.
///
/// It is *not* needed for cross-platform persistence, which was the first
/// reason written here and was wrong: [`Chord::serialize`] maps `cmd_key()` to
/// the platform-resolving `"Mod"` for every variant, `Custom` included, so a
/// `Custom([SuperLeft, S])` reloads correctly on Linux too. Recorded because
/// the plausible-but-false version is what a reader would otherwise re-derive.
fn from_keys(keys: &[KeyCode]) -> Chord {
    let (mods, rest): (Vec<KeyCode>, Vec<KeyCode>) = keys.iter().partition(|k| is_modifier(**k));
    let Some(&key) = rest.first() else {
        return Chord::Custom(keys.to_vec());
    };

    let cmd = mods
        .iter()
        .any(|k| matches!(k, KeyCode::SuperLeft | KeyCode::SuperRight))
        || (!cfg!(target_os = "macos")
            && mods
                .iter()
                .any(|k| matches!(k, KeyCode::ControlLeft | KeyCode::ControlRight)));
    let shift = mods
        .iter()
        .any(|k| matches!(k, KeyCode::ShiftLeft | KeyCode::ShiftRight));
    let alt = mods
        .iter()
        .any(|k| matches!(k, KeyCode::AltLeft | KeyCode::AltRight));

    match (cmd, shift, alt) {
        (false, false, false) => Chord::Key(key),
        (true, false, false) => Chord::Cmd(key),
        (false, true, false) => Chord::Shift(key),
        (true, true, false) => Chord::CmdShift(key),
        (false, false, true) => Chord::Alt(key),
        // No named variant covers the rest (Cmd+Alt, Alt+Shift, …). `Custom`
        // is the honest answer rather than dropping a modifier to force a fit
        // — a silently narrowed chord would bind something the user did not
        // press.
        _ => Chord::Custom(keys.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive one frame with a given press/release set.
    fn app() -> App {
        let mut app = App::new();
        // The system is run directly by `press`/`release` rather than through
        // a schedule, so the plugin's own registration is not what is under
        // test here — `the_recorder_suppresses_dispatch` covers that.
        app.add_plugins(bevy::input::InputPlugin)
            .add_message::<ChordCaptured>();
        app.init_resource::<ChordCapture>();
        app
    }

    // Written *inside* the schedule, ahead of the capture system.
    // `InputPlugin` clears `just_pressed`/`just_released` in `PreUpdate`, so a
    // press applied before `update()` is wiped before the system ever sees it
    // — the first version of these tests did exactly that and failed for a
    // reason that had nothing to do with the recorder.
    fn press(app: &mut App, key: KeyCode) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(key);
        run_capture(app);
    }

    fn release(app: &mut App, key: KeyCode) {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(key);
        run_capture(app);
    }

    /// Run the capture system once against the world's current input state.
    fn run_capture(app: &mut App) {
        if app.world().get_resource::<ChordCapture>().is_none() {
            return;
        }
        app.world_mut()
            .run_system_cached(capture_chord)
            .expect("capture system runs");
    }

    fn captured(app: &mut App) -> Option<ChordCaptured> {
        let messages = app.world().resource::<Messages<ChordCaptured>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).next().cloned()
    }

    #[test]
    fn a_bare_key_is_captured_on_release() {
        let mut app = app();
        press(&mut app, KeyCode::KeyK);
        // Nothing yet — the key is still down, and the user may be about to
        // add to the chord.
        assert!(captured(&mut app).is_none(), "captured before release");

        release(&mut app, KeyCode::KeyK);
        assert!(
            matches!(
                captured(&mut app),
                Some(ChordCaptured::Chord(Chord::Key(KeyCode::KeyK)))
            ),
            "a bare key did not capture"
        );
    }

    #[test]
    fn modifiers_held_across_frames_reach_the_chord() {
        // The reason modifiers accumulate rather than being sampled at the
        // release: they go down on an earlier frame than the key.
        let mut app = app();
        press(&mut app, KeyCode::ShiftLeft);
        press(&mut app, KeyCode::KeyA);
        release(&mut app, KeyCode::KeyA);

        assert!(
            matches!(
                captured(&mut app),
                Some(ChordCaptured::Chord(Chord::Shift(KeyCode::KeyA)))
            ),
            "the modifier was dropped"
        );
    }

    #[test]
    fn a_modifier_alone_never_completes_a_chord() {
        // A binding on Shift alone would fire while typing any capital.
        let mut app = app();
        press(&mut app, KeyCode::ShiftLeft);
        release(&mut app, KeyCode::ShiftLeft);

        assert!(
            captured(&mut app).is_none(),
            "a lone modifier was accepted as a chord"
        );
        assert!(
            app.world().get_resource::<ChordCapture>().is_some(),
            "the recording ended without producing a chord"
        );
    }

    #[test]
    fn escape_cancels_and_is_distinct_from_unbinding() {
        let mut app = app();
        press(&mut app, KeyCode::Escape);

        assert!(
            matches!(captured(&mut app), Some(ChordCaptured::Cancelled)),
            "escape did not cancel"
        );
        assert!(
            app.world().get_resource::<ChordCapture>().is_none(),
            "the recording kept listening after a cancel"
        );
    }

    #[test]
    fn the_recording_stops_after_one_chord() {
        // Without removing the resource the next keystroke anywhere in the app
        // would be swallowed as a second chord.
        let mut app = app();
        press(&mut app, KeyCode::KeyK);
        release(&mut app, KeyCode::KeyK);

        assert!(
            app.world().get_resource::<ChordCapture>().is_none(),
            "the recorder kept listening after capturing"
        );
    }

    #[test]
    fn a_named_variant_is_preferred_over_custom() {
        // So that a captured chord equals a hand-written one. Host code that
        // matches on `Chord::Cmd(..)` would silently miss every recorded
        // binding if capture always produced `Custom`.
        let cmd = if cfg!(target_os = "macos") {
            KeyCode::SuperLeft
        } else {
            KeyCode::ControlLeft
        };
        assert!(matches!(
            from_keys(&[cmd, KeyCode::KeyS]),
            Chord::Cmd(KeyCode::KeyS)
        ));
        assert!(matches!(
            from_keys(&[cmd, KeyCode::ShiftLeft, KeyCode::KeyS]),
            Chord::CmdShift(KeyCode::KeyS)
        ));
        assert!(matches!(
            from_keys(&[KeyCode::AltLeft, KeyCode::KeyS]),
            Chord::Alt(KeyCode::KeyS)
        ));
    }

    #[test]
    fn an_uncovered_modifier_mix_stays_custom() {
        // Cmd+Alt has no named variant. Forcing it into `Cmd` would bind
        // something the user did not press.
        let cmd = if cfg!(target_os = "macos") {
            KeyCode::SuperLeft
        } else {
            KeyCode::ControlLeft
        };
        assert!(matches!(
            from_keys(&[cmd, KeyCode::AltLeft, KeyCode::KeyS]),
            Chord::Custom(_)
        ));
    }

    #[test]
    fn the_right_hand_modifiers_count_too() {
        // A recorder has to accept what the keyboard reports, not the
        // normalised form `Chord::keys` emits.
        assert!(matches!(
            from_keys(&[KeyCode::ShiftRight, KeyCode::KeyA]),
            Chord::Shift(KeyCode::KeyA)
        ));
    }

    #[test]
    fn a_captured_chord_round_trips_through_serialization() {
        // Independent of the variant choice above — `serialize` resolves the
        // command key to "Mod" for `Custom` as well, so this holds either way.
        // Kept because persistence is the thing a user actually notices, and
        // it should be pinned by its own test rather than inferred from the
        // variant one.
        let cmd = if cfg!(target_os = "macos") {
            KeyCode::SuperLeft
        } else {
            KeyCode::ControlLeft
        };
        let chord = from_keys(&[cmd, KeyCode::KeyS]);
        let names = chord.serialize();
        let back = Chord::deserialize(&names).expect("a captured chord must reload");
        assert_eq!(
            back.keys(),
            chord.keys(),
            "a captured chord did not survive a save/load cycle"
        );
    }
}
