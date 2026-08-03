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
    Toast, ToastDismissible, ToastDuration, ToastExternalProgress, ToastMessage, ToastShowProgress,
    ToastTitle, ToastVariant,
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
    external_progress: Option<f32>,
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
            external_progress: None,
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

    /// Externally-driven progress (0.0..=1.0). The toast won't
    /// auto-dismiss until the caller removes
    /// [`ToastExternalProgress`]. Implies [`Self::show_progress`].
    #[must_use]
    pub fn progress(mut self, progress: f32) -> Self {
        self.external_progress = Some(progress.clamp(0.0, 1.0));
        self.show_progress = true;
        self
    }

    /// Show the auto-dismiss countdown as a progress bar. Off by
    /// default (matches shadcn's Sonner).
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
            // Starts fully off-edge; `drive_toasts` targets 1.0.
            Spring::<f32>::new(0.0),
            Name::new("tempera::toast"),
        ));
        if let Some(title) = self.title {
            e.insert(ToastTitle(title));
        }
        if let Some(p) = self.external_progress {
            e.insert(ToastExternalProgress(p));
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
