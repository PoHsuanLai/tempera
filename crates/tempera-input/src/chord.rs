//! Declarative chord spelling, and its serialized form.
//!
//! [`Chord`] is the vocabulary bindings are *written* in. It exists because
//! leafwing's `ButtonlikeChord` requires naming a concrete modifier key, and
//! the concrete key differs per platform: `ButtonlikeChord::new([SuperLeft,
//! KeyS])` is Cmd+S on macOS and a broken binding everywhere else.
//! `Chord::Cmd(KeyS)` is written once and resolves at use.
//!
//! A chord lowers three ways:
//!
//! - [`Chord::as_input`] → leafwing `ButtonlikeChord`, for matching.
//! - `From<Chord> for KbdChord` → tempera keycaps, for display.
//! - [`Chord::serialize`] / [`Chord::deserialize`] → [`SerializedChord`], a
//!   list of stable key *names* for the on-disk format.
//!
//! The serialized form deliberately stores names rather than leafwing or Bevy
//! enum values: a user's saved keybinds must survive a dependency bump that
//! renumbers a `KeyCode`.

use bevy::prelude::*;
use leafwing_input_manager::user_input::ButtonlikeChord;

/// A chord as a list of stable key names, e.g. `["Cmd", "S"]`.
///
/// This is the on-disk shape. Names come from [`keycode_to_name`], which is
/// stable across dependency upgrades in a way that enum discriminants are not.
pub type SerializedChord = Vec<String>;

/// Declarative chord spelling.
///
/// `Cmd` resolves to [`KeyCode::SuperLeft`] on macOS and
/// [`KeyCode::ControlLeft`] elsewhere, so a binding is spelled once and is
/// correct on every platform.
///
/// Not `Copy`: [`Chord::Custom`] owns its key list, because a chord loaded
/// from a user's config is built at runtime and cannot borrow from `'static`.
/// That is the whole reason `Custom` exists alongside [`Chord::Multi`] — the
/// latter keeps `const` spelling cheap for compiled-in defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Chord {
    /// Single key, no modifiers.
    Key(KeyCode),
    /// Cmd+X on macOS, Ctrl+X elsewhere.
    Cmd(KeyCode),
    /// Shift+X.
    Shift(KeyCode),
    /// Cmd+Shift+X (or Ctrl+Shift+X off macOS).
    CmdShift(KeyCode),
    /// Alt/Option+X.
    Alt(KeyCode),
    /// Compiled-in custom chord — the caller supplies the full key list.
    Multi(&'static [KeyCode]),
    /// Runtime-built custom chord, e.g. one loaded from a user's config or
    /// captured by the settings UI's "press a key" recorder.
    Custom(Vec<KeyCode>),
}

impl Chord {
    /// The keys this chord requires, in modifier-then-key order.
    ///
    /// This is the single place the variant → keys mapping lives; every other
    /// lowering ([`as_input`](Self::as_input), display, serialization) goes
    /// through it, so they cannot disagree.
    pub fn keys(&self) -> Vec<KeyCode> {
        match self {
            Chord::Key(k) => vec![*k],
            Chord::Cmd(k) => vec![cmd_key(), *k],
            Chord::Shift(k) => vec![KeyCode::ShiftLeft, *k],
            Chord::CmdShift(k) => vec![cmd_key(), KeyCode::ShiftLeft, *k],
            Chord::Alt(k) => vec![KeyCode::AltLeft, *k],
            Chord::Multi(keys) => keys.to_vec(),
            Chord::Custom(keys) => keys.clone(),
        }
    }

    /// Lower to a leafwing chord for matching.
    pub fn as_input(&self) -> ButtonlikeChord {
        ButtonlikeChord::new(self.keys())
    }

    /// Lower to the on-disk name list.
    ///
    /// The platform-resolving modifier is written as the **logical** name
    /// `"Mod"`, not as the key it happens to resolve to on this machine.
    /// Writing the resolved key would make config files platform-specific:
    /// a `Cmd(S)` binding saved on macOS is `["Cmd","S"]`, which on Linux
    /// would load as a literal Super+S and never fire. `"Mod"` reads back as
    /// "this platform's command modifier" wherever it is loaded.
    ///
    /// Keys with no stable name are dropped, which is why a round-trip is
    /// lossy for exotic keys — see [`keycode_to_name`].
    pub fn serialize(&self) -> SerializedChord {
        let modifier = cmd_key();
        self.keys()
            .into_iter()
            .filter_map(|k| {
                if k == modifier {
                    Some(MOD_NAME.to_owned())
                } else {
                    keycode_to_name(k).map(str::to_owned)
                }
            })
            .collect()
    }

    /// Rebuild a chord from the on-disk name list.
    ///
    /// Returns `None` if the list is empty or every name is unrecognized.
    /// Unknown names are skipped rather than failing the whole chord, so a
    /// config written by a newer build degrades instead of being discarded.
    ///
    /// The result is always [`Chord::Multi`]-shaped in spirit — we cannot know
    /// whether the user meant `Cmd(S)` or `Multi([Super, S])`, and it does not
    /// matter: both produce the same `ButtonlikeChord`. The one thing that
    /// *does* matter is that this returns a `Chord` at all, so a rebound key
    /// can still be *displayed*. (Returning nothing here is what made custom
    /// binds render blank in the original implementation.)
    pub fn deserialize(names: &[String]) -> Option<Chord> {
        let keys: Vec<KeyCode> = names
            .iter()
            .filter_map(|n| {
                if n == MOD_NAME {
                    Some(cmd_key())
                } else {
                    name_to_keycode(n)
                }
            })
            .collect();
        match keys.len() {
            0 => None,
            1 => Some(Chord::Key(keys[0])),
            _ => Some(Chord::Custom(keys)),
        }
    }
}

/// On-disk spelling of the platform-resolving command modifier.
///
/// Deliberately not `"Cmd"` or `"Ctrl"` — both of those are *also* literal
/// key names in [`KEY_NAMES`], and a binding that means "the command key"
/// must be distinguishable from one that means "Control specifically".
pub const MOD_NAME: &str = "Mod";

#[inline]
fn cmd_key() -> KeyCode {
    if cfg!(target_os = "macos") {
        KeyCode::SuperLeft
    } else {
        KeyCode::ControlLeft
    }
}

// ── const constructors ───────────────────────────────────────────────────
//
// `const fn` so bindings can be spelled in const position without a runtime
// allocation.

pub const fn key(k: KeyCode) -> Chord {
    Chord::Key(k)
}
pub const fn cmd(k: KeyCode) -> Chord {
    Chord::Cmd(k)
}
pub const fn shift(k: KeyCode) -> Chord {
    Chord::Shift(k)
}
pub const fn cmd_shift(k: KeyCode) -> Chord {
    Chord::CmdShift(k)
}
pub const fn alt(k: KeyCode) -> Chord {
    Chord::Alt(k)
}
pub const fn multi(keys: &'static [KeyCode]) -> Chord {
    Chord::Multi(keys)
}

// ── KeyCode ↔ stable name ────────────────────────────────────────────────

/// The name table. One list, both directions — a hand-maintained pair of
/// `match` blocks drifts, and a drifted entry means a keybind that saves but
/// never loads.
const KEY_NAMES: &[(KeyCode, &str)] = &[
    (KeyCode::KeyA, "A"),
    (KeyCode::KeyB, "B"),
    (KeyCode::KeyC, "C"),
    (KeyCode::KeyD, "D"),
    (KeyCode::KeyE, "E"),
    (KeyCode::KeyF, "F"),
    (KeyCode::KeyG, "G"),
    (KeyCode::KeyH, "H"),
    (KeyCode::KeyI, "I"),
    (KeyCode::KeyJ, "J"),
    (KeyCode::KeyK, "K"),
    (KeyCode::KeyL, "L"),
    (KeyCode::KeyM, "M"),
    (KeyCode::KeyN, "N"),
    (KeyCode::KeyO, "O"),
    (KeyCode::KeyP, "P"),
    (KeyCode::KeyQ, "Q"),
    (KeyCode::KeyR, "R"),
    (KeyCode::KeyS, "S"),
    (KeyCode::KeyT, "T"),
    (KeyCode::KeyU, "U"),
    (KeyCode::KeyV, "V"),
    (KeyCode::KeyW, "W"),
    (KeyCode::KeyX, "X"),
    (KeyCode::KeyY, "Y"),
    (KeyCode::KeyZ, "Z"),
    (KeyCode::Digit0, "0"),
    (KeyCode::Digit1, "1"),
    (KeyCode::Digit2, "2"),
    (KeyCode::Digit3, "3"),
    (KeyCode::Digit4, "4"),
    (KeyCode::Digit5, "5"),
    (KeyCode::Digit6, "6"),
    (KeyCode::Digit7, "7"),
    (KeyCode::Digit8, "8"),
    (KeyCode::Digit9, "9"),
    (KeyCode::Semicolon, "Semicolon"),
    (KeyCode::Comma, "Comma"),
    (KeyCode::Period, "Period"),
    (KeyCode::Slash, "Slash"),
    (KeyCode::Backslash, "Backslash"),
    (KeyCode::Minus, "Minus"),
    (KeyCode::Equal, "Equal"),
    (KeyCode::BracketLeft, "BracketLeft"),
    (KeyCode::BracketRight, "BracketRight"),
    (KeyCode::Quote, "Quote"),
    (KeyCode::Backquote, "Backquote"),
    (KeyCode::Space, "Space"),
    (KeyCode::Enter, "Enter"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::Backspace, "Backspace"),
    (KeyCode::Delete, "Delete"),
    (KeyCode::Escape, "Escape"),
    (KeyCode::ArrowLeft, "Left"),
    (KeyCode::ArrowRight, "Right"),
    (KeyCode::ArrowUp, "Up"),
    (KeyCode::ArrowDown, "Down"),
    (KeyCode::Home, "Home"),
    (KeyCode::End, "End"),
    (KeyCode::PageUp, "PageUp"),
    (KeyCode::PageDown, "PageDown"),
    (KeyCode::ShiftLeft, "Shift"),
    (KeyCode::ShiftRight, "ShiftRight"),
    (KeyCode::ControlLeft, "Ctrl"),
    (KeyCode::ControlRight, "CtrlRight"),
    (KeyCode::AltLeft, "Alt"),
    (KeyCode::AltRight, "AltRight"),
    (KeyCode::SuperLeft, "Cmd"),
    (KeyCode::SuperRight, "CmdRight"),
    (KeyCode::F1, "F1"),
    (KeyCode::F2, "F2"),
    (KeyCode::F3, "F3"),
    (KeyCode::F4, "F4"),
    (KeyCode::F5, "F5"),
    (KeyCode::F6, "F6"),
    (KeyCode::F7, "F7"),
    (KeyCode::F8, "F8"),
    (KeyCode::F9, "F9"),
    (KeyCode::F10, "F10"),
    (KeyCode::F11, "F11"),
    (KeyCode::F12, "F12"),
];

/// Stable string name for a [`KeyCode`], or `None` for keys we do not bind.
///
/// `None` is not an error: it means the key has no on-disk spelling, so a
/// chord containing it cannot round-trip. Callers drop it rather than failing.
pub fn keycode_to_name(k: KeyCode) -> Option<&'static str> {
    KEY_NAMES.iter().find(|(kc, _)| *kc == k).map(|(_, n)| *n)
}

/// Inverse of [`keycode_to_name`].
pub fn name_to_keycode(name: &str) -> Option<KeyCode> {
    KEY_NAMES
        .iter()
        .find(|(_, n)| *n == name)
        .map(|(kc, _)| *kc)
}

// ── display lowering ─────────────────────────────────────────────────────

/// Resolve a declarative chord into the concrete keys to draw.
///
/// `Chord::keys()` has already picked the platform's modifier, so every segment
/// is a real `KeyCode` by this point — [`key_glyph`](crate::kbd::key_glyph)
/// maps the modifier codes to the same glyphs as the side-agnostic enum, so
/// `Cmd(KeyS)` reads `⌘ S` on macOS and `⌃ S` elsewhere.
impl From<Chord> for crate::kbd::KbdChord {
    fn from(chord: Chord) -> Self {
        use crate::kbd::{KbdChord, KbdKey};
        chord
            .keys()
            .into_iter()
            .fold(KbdChord::new(), |acc, k| acc.with(KbdKey::Key(k)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_name_does_not_collide_with_a_literal_key_name() {
        // If `Mod` were also a literal key name, `deserialize` could not tell
        // "the command key" from that key.
        assert!(name_to_keycode(MOD_NAME).is_none());
    }

    #[test]
    fn name_table_is_a_bijection() {
        // A duplicate on either side silently breaks round-tripping for one
        // of the colliding entries.
        for (kc, name) in KEY_NAMES {
            assert_eq!(
                name_to_keycode(name),
                Some(*kc),
                "name {name} maps back wrong"
            );
            assert_eq!(
                keycode_to_name(*kc),
                Some(*name),
                "keycode {kc:?} maps back wrong"
            );
        }
    }

    #[test]
    fn chord_round_trips_through_the_on_disk_form() {
        for chord in [
            Chord::Key(KeyCode::Escape),
            Chord::Cmd(KeyCode::KeyS),
            Chord::Shift(KeyCode::ArrowUp),
            Chord::CmdShift(KeyCode::KeyZ),
            Chord::Alt(KeyCode::KeyF),
        ] {
            let names = chord.serialize();
            let back = Chord::deserialize(&names).expect("round trip");
            // The *variant* need not survive — `Cmd(S)` and `Multi([Cmd, S])`
            // are the same binding. The key list must.
            assert_eq!(back.keys(), chord.keys(), "{chord:?} lost keys");
        }
    }

    #[test]
    fn deserialize_skips_unknown_names_instead_of_failing() {
        let names = vec![
            MOD_NAME.to_string(),
            "NoSuchKey".to_string(),
            "S".to_string(),
        ];
        let chord = Chord::deserialize(&names).expect("degrades, not fails");
        assert_eq!(chord.keys(), vec![cmd_key(), KeyCode::KeyS]);
    }

    #[test]
    fn the_command_modifier_serializes_platform_independently() {
        // The whole point: a config written on one OS must load correctly on
        // another. `Cmd` must NOT serialize as the key it resolves to here.
        let names = Chord::Cmd(KeyCode::KeyS).serialize();
        assert_eq!(names, vec![MOD_NAME, "S"]);

        // And it must come back as *this* platform's modifier.
        let back = Chord::deserialize(&names).unwrap();
        assert_eq!(back.keys(), vec![cmd_key(), KeyCode::KeyS]);
    }

    #[test]
    fn an_explicit_control_binding_stays_control() {
        // `Mod` means "the command key"; `Ctrl` means Control specifically.
        // Conflating them would silently rewrite deliberate bindings.
        let names = Chord::Multi(&[KeyCode::ControlLeft, KeyCode::KeyS]).serialize();
        if cfg!(target_os = "macos") {
            assert_eq!(names, vec!["Ctrl", "S"]);
            assert_eq!(
                Chord::deserialize(&names).unwrap().keys(),
                vec![KeyCode::ControlLeft, KeyCode::KeyS]
            );
        } else {
            // Off macOS the command key *is* Control, so this is genuinely
            // the same binding and normalizing to `Mod` is correct.
            assert_eq!(names, vec![MOD_NAME, "S"]);
        }
    }

    #[test]
    fn deserialize_of_nothing_usable_is_none() {
        assert!(Chord::deserialize(&[]).is_none());
        assert!(Chord::deserialize(&["NoSuchKey".to_string()]).is_none());
    }
}
