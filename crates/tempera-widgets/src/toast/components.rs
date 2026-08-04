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

/// Where a toast is in its life.
///
/// The toast owns this. A caller reports facts — "this job is 40% done", "it
/// finished" — and the toast decides what that means for its own bar and its
/// own lifetime.
///
/// # Why a named state and not a present-or-absent component
///
/// This replaces `ToastExternalProgress(f32)`, which carried *two* meanings on
/// one component: the fraction to draw, and "do not auto-dismiss". Three things
/// followed from that conflation, and all three are fixed here:
///
/// - **Progress with a timeout was unrepresentable** — any fraction at all
///   suppressed the countdown.
/// - **Indeterminate work was unrepresentable** — holding a toast open with no
///   percentage meant inventing a fake number.
/// - **Finishing resumed a stale countdown.** Completion removed the component,
///   and the timer then ran from [`ToastCreated`] — first appearance, possibly
///   minutes earlier — so a long job's success message could vanish on the
///   frame it appeared. [`Done`](Self::Done) re-stamps the clock instead.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub enum ToastState {
    /// Counting down to auto-dismiss, over [`ToastDuration`] from
    /// [`ToastCreated`]. The progress bar, if shown, tracks the countdown.
    #[default]
    Timed,
    /// Tracking work that has not finished. Never auto-dismisses.
    ///
    /// `None` is indeterminate: the bar renders with nothing to say about how
    /// far along it is. That is honest for work with no measurable total, and
    /// it is what the previous model had no way to express.
    Working { progress: Option<f32> },
    /// The work finished; the toast is now an ordinary timed message.
    ///
    /// Distinct from [`Timed`](Self::Timed) so the tick can tell "never tracked
    /// anything" from "just stopped tracking" — the latter needs its countdown
    /// restarted, and the former must not have it reset from under it.
    Done,
}

impl ToastState {
    /// Whether this state suppresses the auto-dismiss countdown.
    pub fn holds_open(self) -> bool {
        matches!(self, ToastState::Working { .. })
    }

    /// The fraction to draw, if the state has one of its own.
    ///
    /// `Timed` and `Done` answer `None` — their bar tracks the countdown, which
    /// is the tick's business rather than the state's. `Working` with no
    /// progress also answers `None` and means something different: nothing is
    /// known. [`Self::holds_open`] tells the two apart.
    pub fn progress(self) -> Option<f32> {
        match self {
            ToastState::Working { progress } => progress,
            ToastState::Timed | ToastState::Done => None,
        }
    }
}

/// Correlates a toast with a long-running operation, so an update can find it.
///
/// The id is one the caller already has — a job id, a download id, an export
/// id. [`progress`](super::progress) creates the toast on first sight of an id
/// and updates that same entity afterwards, so a caller never holds an
/// `Entity` across frames.
///
/// Looked up by query rather than through a map: the id lives on the toast, so
/// there is nothing to keep in step and nothing to clean up when it despawns.
/// [`complete`](super::complete) removes it, freeing the id for reuse.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ProgressToast(pub String);

/// Marker requesting the progress bar be rendered. Off by default; toasts in
/// [`ToastState::Working`] behave as if this were present.
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

/// Where on the window toasts stack. Defaults to bottom-right.
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
