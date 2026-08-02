//! Keybinds and commands for a content-agnostic application shell.
//!
//! This crate knows nothing about what the host application *does*. It
//! provides the machinery every keyboard-driven app needs — declarative
//! chords, rebindable persistence, and a command registry — and lets the
//! app supply the meaning.
//!
//! # What this adds on top of `leafwing-input-manager`
//!
//! leafwing gives us chord matching (`InputMap<A>` → `ActionState<A>`) and
//! nothing else. On top of that, tempera adds:
//!
//! - [`Chord`] — declarative, platform-aware chord spelling. `Cmd` resolves
//!   to Super on macOS and Control elsewhere, so bindings are written once.
//! - **Commands as entities.** A command carries its own id, label, chord,
//!   handler, and optional condition. One registration feeds the command
//!   palette, the keybinding, the settings UI, and tooltip hints.
//! - **Persistence** keyed on command id (not the action enum), so adding or
//!   removing actions never corrupts a user's saved keybinds.
//! - **Conditions**, so two commands can share a chord and disambiguate on
//!   application state rather than on focus.
//!
//! # Why conditions rather than focus scopes
//!
//! The obvious design — "the focused widget owns a scope, and scopes shadow
//! each other" — cannot express the cases that matter. In VS Code, `Escape`
//! means close-the-suggest-popup, then collapse-multi-cursor, then
//! close-find, then unfocus-terminal: four handlers in the *same* focus
//! context, discriminated by state. Focus scoping has nothing to say there.
//!
//! It is also not implementable against leafwing today. leafwing reads
//! `ButtonInput<KeyCode>`, a global *state* resource, while Bevy's focus
//! bubbling (`bevy_input_focus`) dispatches a *stream* of events. By the time
//! leafwing sees a key, the routing information that focus would filter on is
//! gone — and chords inherently need state, not edges. Bevy's own discussion
//! of this (bevyengine/bevy#15374) has been open since 2024 without a
//! shipped answer.
//!
//! So conditions do the discriminating instead. A condition is an ordinary
//! Bevy run condition — any read-only system returning `bool` — which means
//! `.and()`, `.or()` and `not()` compose for free, and the app writes plain
//! typed systems rather than a string DSL. Focus becomes one condition among
//! many rather than a privileged mechanism.

#![forbid(unsafe_code)]

pub mod binding;
pub mod chord;
pub mod command;
pub mod condition;
pub mod dispatch;
pub mod persist;
pub mod plugin;

pub use chord::{
    Chord, SerializedChord, alt, cmd_shift, key, keycode_to_name, multi, name_to_keycode, shift,
};
// `chord::cmd` (the chord constructor) and `command::cmd` (the bundle helper)
// would collide at the root, and the bundle helper is the one call sites reach
// for constantly. Chords stay reachable as `chord::cmd(..)`.
pub use binding::{Binding, RebindError, rebind, reset_binding, resolve};
pub use chord::cmd as cmd_chord;
pub use command::{
    AppCommandExt, BindScope, Command, CommandId, CommandLabel, CommandMarker, CommandRegistry,
    HeldPayload, Keybind, OnPress, OnRelease, cmd, dyn_cmd, fire, fire_by_id, on_press,
    on_press_held, on_release,
};
pub use condition::{Priority, When};
pub use dispatch::{HeldClaims, dispatch_commands, drain_held_claims};
pub use persist::{SavedKeybinds, ScopeOverrides};
pub use plugin::{AppName, TemperaInputPlugin};
