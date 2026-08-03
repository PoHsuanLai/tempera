//! TextInput — single-line editable text field.
//!
//! Wraps [`bevy_ui_text_input`] (by ickshonpe — the same code being
//! upstreamed into `bevy_ui_widgets` itself). That crate owns the
//! editing behavior (caret movement, selection, clipboard, undo/redo,
//! IME-friendly cosmic-text shaping, validated input modes). Tempera
//! adds the visuals — surround border, hover/focus tint, placeholder
//! styling that matches our theme.
//!
//! ## Composition
//!
//! - [`TextInput`] — tempera marker on the styled root
//! - [`bevy_ui_text_input::TextInputNode`] — behavior + buffer
//! - [`bevy_ui_text_input::TextInputContents`] — change-detected
//!   string mirror of the buffer; observe `Changed<TextInputContents>`
//!   or query directly for the current value
//! - [`bevy_ui_text_input::TextInputPrompt`] — placeholder
//! - [`bevy_ui_text_input::SubmitText`] — message fired on Enter
//!
//! Convenience: tempera re-exports the per-keystroke change as
//! [`ValueChange`] (just a change-detected query helper) so consumers
//! don't have to learn two crates' names.
//!
//! ```ignore
//! use tempera::prelude::*;
//!
//! let id = spawn_text_input(&mut commands, &style, "", "Enter your name…");
//! // React to every edit:
//! commands.add_systems(Update, |q: Query<&TextInputContents, Changed<TextInputContents>>| {
//!     for c in &q { info!("value -> {}", c.get()); }
//! });
//! // Or just react to Enter:
//! commands.add_systems(Update, |mut r: MessageReader<SubmitText>| {
//!     for e in r.read() { info!("submitted -> {}", e.text); }
//! });
//! ```

use bevy::prelude::*;
use bevy_ui_text_input::TextInputPlugin as UpstreamPlugin;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use bevy_ui_text_input::actions::{TextInputAction, TextInputEdit};
pub use bevy_ui_text_input::{
    SubmitText, TextInputBuffer, TextInputContents, TextInputFilter, TextInputMode, TextInputNode,
    TextInputPrompt, TextInputQueue,
};
pub use components::{TextInput, TextInputVariant};
pub use spawn::{
    TextInputHandle, TextInputStyle, spawn_text_input, spawn_text_input_variant,
    spawn_text_input_with_icon,
};

/// Convenience alias for the per-keystroke change-detection pattern.
/// Bevy doesn't have a generic "ValueChange<String>" event for text
/// inputs (the upstream crate uses change detection on
/// [`TextInputContents`] instead), so this type alias gives consumers
/// a query shape that mirrors the rest of tempera's `ValueChange`
/// story.
///
/// ```ignore
/// fn react(q: Query<(Entity, &TextInputContents), tempera::text_input::ValueChange>) { ... }
/// ```
pub type ValueChange = bevy::ecs::query::Changed<TextInputContents>;

pub struct TextInputStylePlugin;

impl Plugin for TextInputStylePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<UpstreamPlugin>() {
            app.add_plugins(UpstreamPlugin);
        }
        // Focus is a third trigger here, alongside interaction and the
        // palette: a border drops back to idle when focus *leaves* it,
        // which is a fact about the resource rather than about the entity
        // that lost it — so it cannot be a query filter either.
        app.add_systems(
            Update,
            systems::repaint_text_input.run_if(
                crate::theme::repaint_needed::<TextInput>
                    .or_else(resource_changed::<bevy::input_focus::InputFocus>),
            ),
        );
    }
}
