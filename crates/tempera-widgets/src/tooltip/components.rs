use bevy::prelude::*;
use tempera_input::CommandId;

use crate::kbd::KbdChord;

/// This tooltip's chord is whatever `0` is bound to, read fresh on each hover.
///
/// Insert it alongside a [`Tooltip`], or let [`Tooltip::shortcut_for`] do it.
///
/// # Why the id is a component and not a field
///
/// [`resolve_command_shortcuts`](super::systems::resolve_command_shortcuts)
/// writes the resolved chord into [`Tooltip::shortcut`], so a field holding the
/// id would be overwritten by its own resolution. The entity remembers instead:
/// this component survives every resolve, and the resolver's query filters on
/// it, so a tooltip with a hand-written chord is never touched.
///
/// # Why the chord is not simply stored at spawn
///
/// That would make the tooltip a second owner of a value
/// [`Keybind`](tempera_input::Keybind) already owns, and the copy would need
/// invalidating on every rebind *and* every unbind. Unbinds are component
/// **removals**, which a `Changed` filter cannot observe — so the version that
/// looks correct is the one that keeps showing a chord the user just cleared.
/// Reading at hover has nothing to invalidate.
///
/// An id nothing claims shows no chord. Commands are registered by whichever
/// crates are present, so a toolbar naming one from an absent crate is an
/// ordinary state rather than a broken one.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct TooltipShortcutFor(pub CommandId);

/// Tooltip preferred placement relative to its target. `Auto` picks
/// whichever direction has room, preferring `Top → Bottom → Right → Left`
/// (matches armas).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipPosition {
    #[default]
    Auto,
    Top,
    Bottom,
    Left,
    Right,
}

/// Attach to a UI node to give it a tooltip. The tooltip popup is
/// spawned on hover (after `delay_ms`) and despawned when the cursor
/// leaves. Position auto-flips if there's not enough room on the
/// preferred side.
#[derive(Component, Clone, Debug)]
pub struct Tooltip {
    pub text: String,
    /// Optional keyboard shortcut. Rendered as a row of `kbd` chips
    /// to the right of the text inside the popup — matches shadcn's
    /// `<TooltipContent>Save Changes <Kbd>S</Kbd></TooltipContent>`
    /// pattern. Typed via [`KbdChord`]; pass a `KeyCode`, a
    /// `ModifierKey`, or a leafwing `ButtonlikeChord`.
    ///
    /// For a chord the user can rebind, add [`TooltipShortcutFor`] instead of
    /// setting this — it is then written for you on each hover.
    pub shortcut: Option<KbdChord>,
    pub position: TooltipPosition,
    /// Wrap width in logical pixels.
    pub max_width: f32,
    /// Hover time before the tooltip appears, in milliseconds.
    /// shadcn defaults to 0 (instant); set higher for less aggressive
    /// reveals.
    pub delay_ms: u64,
    /// Whether to draw the small triangle pointing at the target.
    pub show_arrow: bool,
}

impl Tooltip {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shortcut: None,
            position: TooltipPosition::Auto,
            max_width: 300.0,
            delay_ms: 0,
            show_arrow: true,
        }
    }

    /// Show a fixed chord. Accepts anything that lowers into a
    /// [`KbdChord`] — a [`bevy::input::keyboard::KeyCode`], a leafwing
    /// [`leafwing_input_manager::user_input::keyboard::ModifierKey`], a
    /// `KbdChord` built with `.with(...)`, or a leafwing `ButtonlikeChord`.
    ///
    /// For a chord the user can rebind, spawn [`TooltipShortcutFor`] beside the
    /// tooltip instead — this one is what the caller wrote and never changes.
    #[must_use]
    pub fn shortcut(mut self, chord: impl Into<KbdChord>) -> Self {
        self.shortcut = Some(chord.into());
        self
    }

    #[must_use]
    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    #[must_use]
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = w;
        self
    }

    #[must_use]
    pub fn delay(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    #[must_use]
    pub fn no_arrow(mut self) -> Self {
        self.show_arrow = false;
        self
    }
}

/// Tracks how long the cursor has been over a tooltip target. Inserted
/// on `Pointer<Over>`, removed on `Pointer<Out>`. The spawn system
/// reads `started_at` against the current time to gate the delay.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct TooltipHover {
    pub started_at: f32,
}

/// Marker on the spawned popup node. Carries the target entity so the
/// position-sync system can re-anchor when the target moves, and
/// despawn when the target unmounts.
#[derive(Component, Clone, Copy, Debug)]
pub struct TooltipPopup {
    pub target: Entity,
    pub position: TooltipPosition,
}

/// Marker on the arrow triangle child of a `TooltipPopup`.
#[derive(Component, Default, Debug)]
pub struct TooltipArrow;
