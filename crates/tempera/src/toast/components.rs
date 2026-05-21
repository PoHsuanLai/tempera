use bevy::prelude::*;
use std::time::Duration;

/// Variant of a toast — controls accent color and the leading icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastVariant {
    #[default]
    Default,
    Destructive,
}

/// Where on the window toasts stack. shadcn's Sonner default is
/// bottom-right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

/// Opaque handle returned when creating a toast. Use with
/// `ToastManager::set_progress` / `set_message` / `start_dismiss` /
/// `dismiss` to update or remove a toast after it's been added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(pub(crate) u64);

/// Marker on the toast root entity. Carries the toast id so update
/// systems can find it.
#[derive(Component, Clone, Copy, Debug)]
pub struct ToastNode {
    pub id: u64,
}

/// Marker on the toast's message Text child. Used by `set_message`
/// to update the displayed text in place.
#[derive(Component, Debug)]
pub struct ToastMessageText;

/// Marker on the toast's title Text child.
#[derive(Component, Debug)]
pub struct ToastTitleText;

/// Marker on the progress-bar fill child. Width is updated each
/// frame to reflect either the auto-dismiss countdown or the
/// externally-driven progress value. The drive system reaches it
/// via the entity stored on the `ToastRecord`, not by querying for
/// this marker — the marker is just a name tag.
#[derive(Component, Default, Debug)]
pub struct ToastProgressFill;

/// Configuration for spawning a single toast. Stored in the
/// [`crate::toast::ToastManager`] queue until rendered.
#[derive(Clone, Debug)]
pub struct ToastConfig {
    pub id: u64,
    pub title: Option<String>,
    pub message: String,
    pub variant: ToastVariant,
    pub duration: Duration,
    pub dismissible: bool,
    /// If `Some(_)`, the toast won't auto-dismiss; the progress bar
    /// reflects this externally-driven value (0.0..=1.0). Use
    /// `ToastManager::start_dismiss` to flip back to the timed
    /// countdown. Always rendered with a visible progress bar.
    pub external_progress: Option<f32>,
    /// Whether to render the auto-dismiss countdown as a progress bar
    /// underneath the message. Defaults to `false` (matches shadcn's
    /// Sonner). External-progress toasts always show the bar
    /// regardless.
    pub show_progress: bool,
}
