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

/// Tracks whether tempera's cursor system is the current owner of the
/// primary window's [`CursorIcon`]. Used by [`drive_window_cursor`] to
/// release ownership exactly once when the last hovered widget goes
/// idle, instead of stomping the window cursor every frame.
#[derive(Resource, Default)]
pub(crate) struct WindowCursorOwned(bool);

/// Write the appropriate cursor onto the primary window when a tempera
/// widget is hovered.
///
/// Tempera writes only while it has something to say. When a
/// `HoverCursor`-tagged entity is hovered or pressed, it owns the
/// window cursor and writes its preferred icon. When the last hovered
/// widget goes idle, tempera releases ownership by writing
/// `SystemCursorIcon::Default` exactly once — after that the window is
/// left alone so any other system in the app (editor cursor, custom
/// host cursor, etc.) can drive it without fighting over the value
/// every frame.
pub(crate) fn drive_window_cursor(
    hovered: Query<(&Interaction, &HoverCursor)>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut owned: ResMut<WindowCursorOwned>,
    mut commands: Commands,
) {
    let Ok(window) = primary.single() else {
        return;
    };

    let active = hovered.iter().find_map(|(interaction, cursor)| {
        matches!(interaction, Interaction::Hovered | Interaction::Pressed).then_some(cursor.0)
    });

    match (active, owned.0) {
        (Some(icon), _) => {
            commands.entity(window).insert(CursorIcon::System(icon));
            owned.0 = true;
        }
        (None, true) => {
            // Release: write Default once, then back off so others can drive.
            commands
                .entity(window)
                .insert(CursorIcon::System(SystemCursorIcon::Default));
            owned.0 = false;
        }
        (None, false) => {}
    }
}

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WindowCursorOwned>();
        app.add_systems(Update, drive_window_cursor);
    }
}
