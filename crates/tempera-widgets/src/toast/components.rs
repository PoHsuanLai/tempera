use bevy::prelude::*;
use std::time::Duration;

/// Marker on a toast root entity. Carry alongside per-field
/// components ([`ToastVariant`], [`ToastMessage`], …).
#[derive(Component, Default, Debug)]
pub struct Toast;

/// Variant of a toast — controls accent color and the leading icon.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastVariant {
    #[default]
    Default,
    Destructive,
}

/// Optional title shown bold above the message. Spawning without this
/// component renders a message-only toast.
#[derive(Component, Clone, Debug, Default)]
pub struct ToastTitle(pub String);

/// Body text. Mutate in place — the reconcile system writes the new
/// string into the existing message text node on `Changed<ToastMessage>`.
#[derive(Component, Clone, Debug, Default)]
pub struct ToastMessage(pub String);

/// Timestamp written by the reconcile system the first frame the toast
/// is processed. Used to compute the auto-dismiss countdown.
#[derive(Component, Clone, Copy, Debug)]
pub struct ToastCreated(pub f32);

/// Auto-dismiss timeout. Ignored while [`ToastExternalProgress`] is set.
#[derive(Component, Clone, Copy, Debug)]
pub struct ToastDuration(pub Duration);

impl Default for ToastDuration {
    fn default() -> Self {
        Self(Duration::from_secs_f32(4.0))
    }
}

/// Externally-driven progress (0.0..=1.0). While present, the toast
/// will not auto-dismiss; the progress bar reflects this value
/// instead of the countdown. Remove the component to flip back to the
/// timed countdown ([`commands.entity(e).remove::<ToastExternalProgress>()`]).
#[derive(Component, Clone, Copy, Debug)]
pub struct ToastExternalProgress(pub f32);

/// Marker requesting the progress bar be rendered (defaults off, to
/// match shadcn's Sonner). External-progress toasts implicitly behave
/// as if this were present.
#[derive(Component, Default, Debug)]
pub struct ToastShowProgress;

/// Marker — the toast can be dismissed by clicking. Reserved; the
/// click handler is not wired yet.
#[derive(Component, Default, Debug)]
pub struct ToastDismissible;

// The slide-in animation state is a plain `crate::anim::Spring<f32>` on
// the toast entity. It used to be a `ToastSlide` newtype with the same
// two fields, which meant copying state into a temporary spring and back
// out every frame. `Spring` is itself a `Component`, so the newtype was
// buying nothing.

/// UI subtree handles, written by the reconcile system once it spawns
/// the toast's node tree. Subsequent frames look these up to update
/// the message text and progress-bar fill width without a query walk.
#[derive(Component, Clone, Copy, Debug)]
pub struct ToastNodes {
    pub root: Entity,
    pub message_text: Entity,
    pub title_text: Option<Entity>,
    pub progress_fill: Option<Entity>,
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
