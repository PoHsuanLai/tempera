//! Toast spawn API.
//!
//! Spawning a toast is just spawning an entity. The [`ToastSpec`]
//! builder is sugar for the common cases (title, message, variant,
//! progress, custom duration); the free [`spawn`] / [`spawn_error`]
//! helpers cover the one-liners.
//!
//! ```ignore
//! // One-shot.
//! tempera::toast::spawn(&mut commands, "Project saved");
//!
//! // Destructive variant.
//! tempera::toast::spawn_error(&mut commands, "Save failed");
//!
//! // Progress toast — caller keeps the Entity to update later.
//! let e = tempera::toast::ToastSpec::new("Rendering audio…")
//!     .title("Exporting")
//!     .progress(0.0)
//!     .spawn(&mut commands);
//!
//! // Later: update the message + progress.
//! commands.entity(e)
//!     .insert(ToastMessage("Halfway".into()))
//!     .insert(ToastExternalProgress(0.5));
//!
//! // Finish + start the timed countdown back up.
//! commands.entity(e)
//!     .insert(ToastMessage("Done!".into()))
//!     .remove::<ToastExternalProgress>();
//! ```

use std::time::Duration;

use bevy::prelude::*;

use super::components::{
    ProgressToast, Toast, ToastDismissible, ToastDuration, ToastMessage, ToastShowProgress,
    ToastState, ToastTitle, ToastVariant,
};
use crate::anim::Spring;

/// Builder for a toast entity. Spawn with [`ToastSpec::spawn`].
#[derive(Clone, Debug)]
pub struct ToastSpec {
    title: Option<String>,
    message: String,
    variant: ToastVariant,
    duration: Duration,
    dismissible: bool,
    state: ToastState,
    show_progress: bool,
}

impl ToastSpec {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            title: None,
            message: message.into(),
            variant: ToastVariant::Default,
            duration: ToastDuration::default().0,
            dismissible: true,
            state: ToastState::Timed,
            show_progress: false,
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    #[must_use]
    pub fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn destructive(mut self) -> Self {
        self.variant = ToastVariant::Destructive;
        self
    }

    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    #[must_use]
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Track work at `progress` (0.0..=1.0). The toast will not auto-dismiss
    /// until something moves it out of [`ToastState::Working`]. Implies
    /// [`Self::show_progress`].
    ///
    /// For work identified by an id, prefer [`progress`](super::progress) — it
    /// creates and updates the same toast without the caller holding an
    /// `Entity` across frames.
    #[must_use]
    pub fn progress(mut self, progress: f32) -> Self {
        self.state = ToastState::Working {
            progress: Some(progress.clamp(0.0, 1.0)),
        };
        self.show_progress = true;
        self
    }

    /// Track work with no measurable total. The bar renders; nothing claims to
    /// know how far along it is.
    #[must_use]
    pub fn indeterminate(mut self) -> Self {
        self.state = ToastState::Working { progress: None };
        self.show_progress = true;
        self
    }

    /// Show the auto-dismiss countdown as a progress bar. Off by default.
    #[must_use]
    pub fn show_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Spawn the toast entity. Returns its [`Entity`] for follow-up
    /// mutations.
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let mut e = commands.spawn((
            Toast,
            self.variant,
            ToastMessage(self.message),
            ToastDuration(self.duration),
            self.state,
            // Starts fully off-edge; `drive_toasts` targets 1.0.
            Spring::<f32>::new(0.0),
            Name::new("tempera::toast"),
        ));
        if let Some(title) = self.title {
            e.insert(ToastTitle(title));
        }
        if self.show_progress {
            e.insert(ToastShowProgress);
        }
        if self.dismissible {
            e.insert(ToastDismissible);
        }
        e.id()
    }
}

/// Spawn a one-shot default-variant toast.
pub fn spawn(commands: &mut Commands, message: impl Into<String>) -> Entity {
    ToastSpec::new(message).spawn(commands)
}

/// Spawn a destructive (error) toast.
pub fn spawn_error(commands: &mut Commands, message: impl Into<String>) -> Entity {
    ToastSpec::new(message).destructive().spawn(commands)
}

/// Report progress for the operation called `op_id`, creating its toast on the
/// first call and updating that same toast afterwards.
///
/// `fraction` is `None` for indeterminate work — the bar renders with nothing
/// to say about how far along it is.
///
/// The caller holds no `Entity` between calls, which is the point: a long job
/// reports against an id it already has, from wherever it happens to be, and
/// nothing has to be threaded through in between. A toast tracking work does
/// not auto-dismiss; finish it with [`complete`].
pub fn progress(
    commands: &mut Commands,
    op_id: impl Into<String>,
    message: impl Into<String>,
    fraction: Option<f32>,
) {
    let op_id = op_id.into();
    let message = message.into();
    // Deferred because this has to *find* an existing toast, and a caller
    // should not have to take a `Query<&ProgressToast>` to report a number.
    commands.queue(move |world: &mut World| {
        let state = ToastState::Working {
            progress: fraction.map(|f| f.clamp(0.0, 1.0)),
        };
        match find_by_op_id(world, &op_id) {
            Some(entity) => {
                world
                    .entity_mut(entity)
                    .insert((ToastMessage(message), state));
            }
            None => {
                let entity = world
                    .spawn((
                        Toast,
                        ToastVariant::Default,
                        ToastMessage(message),
                        ToastDuration::default(),
                        state,
                        ToastShowProgress,
                        ToastDismissible,
                        Spring::<f32>::new(0.0),
                        ProgressToast(op_id),
                        Name::new("tempera::toast"),
                    ))
                    .id();
                let _ = entity;
            }
        }
    });
}

/// Finish the operation called `op_id`: show `message`, stop holding the toast
/// open, and start its dismissal countdown from now.
///
/// `success = false` switches it to the destructive variant first.
///
/// If no toast matches — the operation finished faster than its first progress
/// report, or the toast was already dismissed — this spawns a one-shot instead,
/// so the outcome is never silently dropped.
///
/// The id is released, so the same one may be reused for later work.
pub fn complete(
    commands: &mut Commands,
    op_id: impl Into<String>,
    message: impl Into<String>,
    success: bool,
) {
    let op_id = op_id.into();
    let message = message.into();
    commands.queue(move |world: &mut World| {
        let variant = if success {
            ToastVariant::Default
        } else {
            ToastVariant::Destructive
        };
        match find_by_op_id(world, &op_id) {
            Some(entity) => {
                let mut toast = world.entity_mut(entity);
                toast.insert((ToastMessage(message), variant, ToastState::Done));
                // The id is free for reuse the moment the work behind it is
                // over; leaving it would make a later operation with the same
                // id adopt this finished toast.
                toast.remove::<ProgressToast>();
            }
            None => {
                world.spawn((
                    Toast,
                    variant,
                    ToastMessage(message),
                    ToastDuration::default(),
                    ToastState::Timed,
                    ToastDismissible,
                    Spring::<f32>::new(0.0),
                    Name::new("tempera::toast"),
                ));
            }
        }
    });
}

/// The live toast tracking `op_id`, if there is one.
fn find_by_op_id(world: &mut World, op_id: &str) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &ProgressToast), With<Toast>>()
        .iter(world)
        .find(|(_, tracked)| tracked.0 == op_id)
        .map(|(entity, _)| entity)
}
