//! Keyboard-shortcut chip. Renders a typed [`KbdChord`] as a row of
//! rounded chips, one per modifier or key, with Mac-style glyphs
//! (`⌘`, `⇧`, …) for modifiers and arrow keys.
//!
//! Input is typed — pass a [`bevy::input::keyboard::KeyCode`], a
//! [`leafwing_input_manager::user_input::ButtonlikeChord`], or
//! tempera's own [`KbdChord`] builder. No stringly-typed chords.

use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use leafwing_input_manager::user_input::keyboard::ModifierKey;
use leafwing_input_manager::user_input::ButtonlikeChord;
use leafwing_input_manager::user_input::UserInput;

use crate::theme::{ColorPalette, FontHandle, ThemePlugin, Typography};

#[derive(SystemParam)]
pub struct KbdStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
}

/// A typed key segment that knows how to render itself as a single
/// chip. Modifiers come first in display order (`Cmd Shift S`),
/// then non-modifier keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KbdKey {
    Modifier(ModifierKey),
    Key(KeyCode),
}

impl KbdKey {
    /// The rendered text for this chip. Modifiers map to Mac glyphs,
    /// arrows / common named keys also map to single glyphs, and
    /// letters/digits return their plain label (`"S"`, `"5"`).
    pub fn glyph(self) -> String {
        match self {
            KbdKey::Modifier(m) => modifier_glyph(m).to_string(),
            KbdKey::Key(k) => key_glyph(k).to_string(),
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

/// A keyboard chord — modifiers + a primary key — to render as a row
/// of chips. Internally just a `Vec<KbdKey>`; build via [`Self::new`],
/// the various `From` impls, or by lowering a leafwing
/// [`ButtonlikeChord`].
#[derive(Clone, Debug, Default)]
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

    /// Iterate the segments in render order (modifiers first, then
    /// non-modifier keys — independent of insertion order).
    pub fn render_order(&self) -> impl Iterator<Item = &KbdKey> {
        // Modifiers first.
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

/// Lower leafwing's runtime chord type into a display-friendly
/// `KbdChord`. Each `Box<dyn Buttonlike>` segment is downcast to
/// either `ModifierKey` or `KeyCode`; anything else (gamepad,
/// mousewheel) is silently dropped — those are out of scope for
/// keyboard-shortcut display.
impl From<&ButtonlikeChord> for KbdChord {
    fn from(chord: &ButtonlikeChord) -> Self {
        let mut out = KbdChord::new();
        // `decompose()` returns a `BasicInputs::Chord(Vec<Box<dyn
        // Buttonlike>>)` for a `ButtonlikeChord`; `.inputs()`
        // exposes the boxed segments without touching the private
        // inner `Vec`.
        for boxed in chord.decompose().inputs() {
            // Every Buttonlike implements Reflect — downcast.
            let reflect = boxed.as_reflect();
            if let Some(m) = reflect.downcast_ref::<ModifierKey>() {
                out.0.push(KbdKey::Modifier(*m));
            } else if let Some(k) = reflect.downcast_ref::<KeyCode>() {
                out.0.push(KbdKey::Key(*k));
            }
        }
        out
    }
}

impl From<ButtonlikeChord> for KbdChord {
    fn from(chord: ButtonlikeChord) -> Self {
        Self::from(&chord)
    }
}

/// Spawn a row of styled chips for a `KbdChord`. Returns the row
/// entity so the caller can re-parent it.
pub fn spawn_kbd(commands: &mut Commands, style: &KbdStyle, chord: impl Into<KbdChord>) -> Entity {
    let chord: KbdChord = chord.into();
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new("tempera::kbd"),
        ))
        .id();

    for segment in chord.render_order() {
        let display = segment.glyph();
        commands
            .spawn((
                Node {
                    padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(style.palette.muted),
                BorderColor::all(style.palette.border),
                ChildOf(row),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(display),
                    style.font.text_font(style.typography.xs),
                    TextColor(style.palette.muted_foreground),
                    bevy::picking::Pickable::IGNORE,
                ));
            });
    }

    row
}

/// Modifier glyph for a leafwing `ModifierKey`.
pub fn modifier_glyph(m: ModifierKey) -> &'static str {
    match m {
        ModifierKey::Alt => "⌥",
        ModifierKey::Control => "⌃",
        ModifierKey::Shift => "⇧",
        ModifierKey::Super => "⌘",
    }
}

/// Display text for a `KeyCode`. Modifier variants of `KeyCode`
/// (e.g. `AltLeft`) map to the same glyphs as the side-agnostic
/// `ModifierKey` enum, so you get the same look whether your binding
/// resolves to a side-specific code or a generic modifier.
pub fn key_glyph(k: KeyCode) -> String {
    use KeyCode::*;
    let s: &str = match k {
        // Modifier sides — map to Mac glyphs.
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

        // Letters / digits — strip the `Key`/`Digit` prefix from the
        // Debug repr (`KeyS` → `S`, `Digit5` → `5`). Anything we
        // don't recognise falls through to its Debug form.
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

pub struct KbdPlugin;

impl Plugin for KbdPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
    }
}
