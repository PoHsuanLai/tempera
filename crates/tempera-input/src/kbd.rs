//! How a chord is spelled on screen.
//!
//! [`Chord`](crate::Chord) is *declarative* — `Chord::Cmd(KeyS)` is Cmd+S on
//! macOS and Ctrl+S everywhere else, resolved at use. [`KbdChord`] is what that
//! resolves **to** for display: a flat list of concrete segments, each knowing
//! its own glyph.
//!
//! The two are not duplicates and neither replaces the other. A binding is
//! stored declaratively so it survives crossing platforms; a rendered keycap
//! has to name the actual key the user will press.
//!
//! # Why this lives here and not with the widget that draws it
//!
//! `⌘` for [`ModifierKey::Super`] and `⏎` for [`KeyCode::Enter`] is keyboard
//! vocabulary, not styling — nothing about it depends on a palette, a font or a
//! layout. It sat in `tempera-widgets` beside `spawn_kbd`, which meant
//! `tempera-input` depended on the whole widget library to name one `Vec`, and
//! made a *keycap* the only thing that could describe a key.
//!
//! Now the vocabulary is here and the widgets read it. `spawn_kbd` and
//! `spawn_chord_inline` stay in `tempera-widgets` — those genuinely are
//! rendering, and they take a [`KbdChord`] like any other input.

use bevy::input::keyboard::KeyCode;
use leafwing_input_manager::user_input::keyboard::ModifierKey;
use leafwing_input_manager::user_input::{ButtonlikeChord, UserInput};

/// One segment of a chord: a modifier, or a key.
///
/// `Copy` because it is two enum discriminants wide and gets passed around
/// during rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KbdKey {
    Modifier(ModifierKey),
    Key(KeyCode),
}

impl KbdKey {
    /// The text to draw for this segment.
    ///
    /// Modifiers map to Mac glyphs, arrows and common named keys to single
    /// glyphs, and letters/digits to their plain label (`"S"`, `"5"`).
    pub fn glyph(self) -> String {
        match self {
            KbdKey::Modifier(m) => modifier_glyph(m).to_string(),
            KbdKey::Key(k) => key_glyph(k),
        }
    }
}

impl From<ModifierKey> for KbdKey {
    fn from(m: ModifierKey) -> Self {
        KbdKey::Modifier(m)
    }
}

impl From<KeyCode> for KbdKey {
    fn from(k: KeyCode) -> Self {
        KbdKey::Key(k)
    }
}

/// A chord as a list of drawable segments.
///
/// Build it with [`Self::new`] plus [`Self::with`], from a single `KeyCode` or
/// `ModifierKey` via `From`, or by lowering a declarative
/// [`Chord`](crate::Chord) or a leafwing [`ButtonlikeChord`].
///
/// `PartialEq` so a consumer can tell whether the chord it is showing is still
/// the chord that is bound — comparing the rendered form is the only way to ask
/// that without re-deriving both sides.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KbdChord(pub Vec<KbdKey>);

impl KbdChord {
    /// Empty chord.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Single key, no modifiers.
    pub fn key(k: KeyCode) -> Self {
        Self(vec![KbdKey::Key(k)])
    }

    /// Append a modifier or key.
    #[must_use]
    pub fn with(mut self, segment: impl Into<KbdKey>) -> Self {
        self.0.push(segment.into());
        self
    }

    /// The segments in display order — modifiers first, then keys —
    /// independent of the order they were added in.
    pub fn render_order(&self) -> impl Iterator<Item = &KbdKey> {
        let (mods, keys): (Vec<&KbdKey>, Vec<&KbdKey>) = self
            .0
            .iter()
            .partition(|k| matches!(k, KbdKey::Modifier(_)));
        mods.into_iter().chain(keys)
    }
}

impl From<KeyCode> for KbdChord {
    fn from(k: KeyCode) -> Self {
        Self::key(k)
    }
}

impl From<ModifierKey> for KbdChord {
    fn from(m: ModifierKey) -> Self {
        Self(vec![KbdKey::Modifier(m)])
    }
}

impl From<KbdKey> for KbdChord {
    fn from(k: KbdKey) -> Self {
        Self(vec![k])
    }
}

/// Lower leafwing's runtime chord into a drawable one.
///
/// Segments that are not keyboard keys — gamepad, mouse wheel — are dropped,
/// being outside what a keycap can show.
///
/// # A modifier arrives twice
///
/// `decompose()` expands [`ModifierKey::Super`] into **both** `SuperLeft` and
/// `SuperRight`, and hands back plain `KeyCode`s — a `ModifierKey` never
/// survives the call. Pushing each segment as it comes therefore renders
/// `⌘ ⌘ S` for what the user pressed as `⌘S`, because both sides map to the
/// same glyph.
///
/// So the two sides are folded back into the side-agnostic [`KbdKey::Modifier`]
/// they came from, and a modifier already present is not repeated. A binding
/// deliberately naming one side — a keyboard where the two do different things
/// — still renders, as that side's own glyph.
impl From<&ButtonlikeChord> for KbdChord {
    fn from(chord: &ButtonlikeChord) -> Self {
        let mut out = KbdChord::new();
        // `decompose()` yields `BasicInputs::Chord(Vec<Box<dyn Buttonlike>>)`;
        // `.inputs()` exposes the boxed segments without touching the private
        // inner `Vec`.
        for boxed in chord.decompose().inputs() {
            // Every `Buttonlike` implements `Reflect`.
            let reflect = boxed.as_reflect();
            let segment = if let Some(m) = reflect.downcast_ref::<ModifierKey>() {
                KbdKey::Modifier(*m)
            } else if let Some(k) = reflect.downcast_ref::<KeyCode>() {
                modifier_of(*k).map_or(KbdKey::Key(*k), KbdKey::Modifier)
            } else {
                continue;
            };
            if !out.0.contains(&segment) {
                out.0.push(segment);
            }
        }
        out
    }
}

/// The side-agnostic modifier a `KeyCode` is one side of, if it is one.
///
/// Only used when lowering: elsewhere a side-specific code is left alone, since
/// [`key_glyph`] already draws it the same.
fn modifier_of(k: KeyCode) -> Option<ModifierKey> {
    match k {
        KeyCode::SuperLeft | KeyCode::SuperRight => Some(ModifierKey::Super),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(ModifierKey::Shift),
        KeyCode::AltLeft | KeyCode::AltRight => Some(ModifierKey::Alt),
        KeyCode::ControlLeft | KeyCode::ControlRight => Some(ModifierKey::Control),
        _ => None,
    }
}

impl From<ButtonlikeChord> for KbdChord {
    fn from(chord: ButtonlikeChord) -> Self {
        Self::from(&chord)
    }
}

/// Glyph for a side-agnostic modifier.
pub fn modifier_glyph(m: ModifierKey) -> &'static str {
    match m {
        ModifierKey::Alt => "⌥",
        ModifierKey::Control => "⌃",
        ModifierKey::Shift => "⇧",
        ModifierKey::Super => "⌘",
    }
}

/// Display text for a `KeyCode`.
///
/// The side-specific modifier codes (`AltLeft`, …) map to the same glyphs as
/// the side-agnostic [`ModifierKey`] enum, so a binding looks the same whether
/// it resolved to one or the other.
pub fn key_glyph(k: KeyCode) -> String {
    use KeyCode::*;
    let s: &str = match k {
        // Modifier sides — same glyphs as `ModifierKey`.
        SuperLeft | SuperRight => "⌘",
        ShiftLeft | ShiftRight => "⇧",
        AltLeft | AltRight => "⌥",
        ControlLeft | ControlRight => "⌃",

        // Common named keys.
        Enter | NumpadEnter => "⏎",
        Backspace => "⌫",
        Delete => "⌦",
        Escape => "⎋",
        Tab => "⇥",
        CapsLock => "⇪",
        ArrowUp => "↑",
        ArrowDown => "↓",
        ArrowLeft => "←",
        ArrowRight => "→",
        PageUp => "⇞",
        PageDown => "⇟",
        Home => "↖",
        End => "↘",
        Space => "␣",

        // Letters and digits — strip the `Key`/`Digit` prefix from the Debug
        // repr (`KeyS` → `S`, `Digit5` → `5`). Anything unrecognised falls
        // through to its Debug form.
        _ => return strip_key_prefix(format!("{k:?}")),
    };
    s.to_string()
}

fn strip_key_prefix(s: String) -> String {
    if let Some(rest) = s.strip_prefix("Key") {
        return rest.to_string();
    }
    if let Some(rest) = s.strip_prefix("Digit") {
        return rest.to_string();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_render_before_keys_whatever_the_build_order() {
        // `Cmd S`, never `S Cmd` — the reading order is a property of the
        // chord, not of how the caller happened to assemble it.
        let chord = KbdChord::new().with(KeyCode::KeyS).with(ModifierKey::Super);

        let glyphs: Vec<String> = chord.render_order().map(|k| k.glyph()).collect();
        assert_eq!(glyphs, ["⌘", "S"]);
    }

    #[test]
    fn a_letter_loses_its_keycode_prefix() {
        assert_eq!(KbdKey::Key(KeyCode::KeyS).glyph(), "S");
        assert_eq!(KbdKey::Key(KeyCode::Digit5).glyph(), "5");
    }

    #[test]
    fn both_spellings_of_a_modifier_draw_the_same() {
        // A binding that resolved to a side-specific code must not look
        // different from one that stayed side-agnostic.
        assert_eq!(
            KbdKey::Key(KeyCode::SuperLeft).glyph(),
            KbdKey::Modifier(ModifierKey::Super).glyph()
        );
    }

    #[test]
    fn an_unnamed_key_falls_back_to_its_debug_form() {
        // Better a readable "F13" than an empty chip.
        assert_eq!(KbdKey::Key(KeyCode::F13).glyph(), "F13");
    }

    #[test]
    fn a_leafwing_chord_lowers_to_its_segments() {
        let chord: KbdChord = ButtonlikeChord::new([ModifierKey::Super])
            .with(KeyCode::KeyS)
            .into();

        let glyphs: Vec<String> = chord.render_order().map(|k| k.glyph()).collect();
        assert_eq!(glyphs, ["⌘", "S"]);
    }

    #[test]
    fn a_modifier_is_one_chip_not_two() {
        // `decompose()` expands `Super` into `SuperLeft` *and* `SuperRight`,
        // both of which draw `⌘`. Taken at face value the user sees `⌘ ⌘ S`
        // for what they pressed as `⌘S`.
        let chord: KbdChord = ButtonlikeChord::new([ModifierKey::Super])
            .with(KeyCode::KeyS)
            .into();

        assert_eq!(
            chord.0.len(),
            2,
            "one modifier and one key, not a chip per physical side: {chord:?}"
        );
    }

    #[test]
    fn two_different_modifiers_both_survive_the_dedup() {
        // The dedup must key on *which* modifier, not on "is a modifier" —
        // collapsing by kind would silently turn `⌘⇧S` into `⌘S`.
        let chord: KbdChord = ButtonlikeChord::new([ModifierKey::Super, ModifierKey::Shift])
            .with(KeyCode::KeyS)
            .into();

        let glyphs: Vec<String> = chord.render_order().map(|k| k.glyph()).collect();
        assert_eq!(glyphs.len(), 3, "got {glyphs:?}");
        assert!(glyphs.contains(&"⌘".to_string()));
        assert!(glyphs.contains(&"⇧".to_string()));
    }
}
