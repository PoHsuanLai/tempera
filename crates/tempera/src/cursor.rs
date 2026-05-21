//! Cursor-on-hover plumbing.
//!
//! Bevy's `bevy_winit` reads the [`CursorIcon`] component **from the
//! primary `Window` entity**, not from arbitrary UI nodes. So a
//! "change the cursor when hovering this widget" pattern requires a
//! small system that copies a per-widget desired cursor into the
//! Window. Tempera ships that here.
//!
//! ## How widgets opt in
//!
//! Attach a [`HoverCursor`] component to any UI entity carrying
//! `Interaction`. When the pointer enters (`Interaction::Hovered` or
//! `Interaction::Pressed`), tempera writes that cursor onto the
//! primary window. When all `HoverCursor`-tagged entities are idle,
//! the window cursor resets to [`SystemCursorIcon::Default`].
//!
//! Tempera's spawn helpers attach this automatically for the
//! interactive widgets (buttons, checkboxes, sliders, tab triggers,
//! toggle items, dropdown triggers).

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

/// Desired cursor icon while the pointer is over this entity.
///
/// Tempera's spawn helpers default this to
/// `SystemCursorIcon::Pointer` for buttons / toggles / sliders, so a
/// hand cursor appears on hover the way the rest of the OS works.
#[derive(Component, Clone, Copy, Debug)]
pub struct HoverCursor(pub SystemCursorIcon);

impl Default for HoverCursor {
    fn default() -> Self {
        Self(SystemCursorIcon::Pointer)
    }
}

/// Write the appropriate cursor onto the primary window each frame:
/// the cursor of the most-recently-hovered tagged entity, or
/// `Default` if none are hovered.
pub(crate) fn drive_window_cursor(
    hovered: Query<(&Interaction, &HoverCursor)>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Ok(window) = primary.single() else {
        return;
    };

    let mut desired = SystemCursorIcon::Default;
    for (interaction, cursor) in &hovered {
        if matches!(interaction, Interaction::Hovered | Interaction::Pressed) {
            desired = cursor.0;
            break;
        }
    }

    commands
        .entity(window)
        .insert(CursorIcon::System(desired));
}

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, drive_window_cursor);
    }
}
