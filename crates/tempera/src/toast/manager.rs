use bevy::prelude::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::components::{ToastConfig, ToastId, ToastPosition, ToastVariant};

pub(crate) const DEFAULT_DURATION_SECS: f32 = 4.0;
pub(crate) const MAX_TOASTS: usize = 5;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Internal record of an active toast — config plus the runtime state
/// the render system needs (spawn time, slide animation value,
/// already-spawned entity if any).
#[derive(Debug)]
pub(crate) struct ToastRecord {
    pub config: ToastConfig,
    pub created_at: f32,
    /// Spring value 0.0 → 1.0 driving the slide-in offset.
    pub slide: f32,
    pub slide_velocity: f32,
    /// `Some(entity)` once the UI node has been spawned by the render
    /// system. Used to update / despawn the node without re-spawning.
    pub node: Option<Entity>,
    /// `Some(entity)` for the progress-bar fill child if the toast
    /// was spawned with one. The drive system mutates this entity's
    /// `Node.width` each frame.
    pub progress_fill: Option<Entity>,
}

impl ToastRecord {
    fn new(config: ToastConfig) -> Self {
        Self {
            config,
            // The render system overwrites this on first paint.
            created_at: -1.0,
            slide: 0.0,
            slide_velocity: 0.0,
            node: None,
            progress_fill: None,
        }
    }

    /// True when the timed countdown has elapsed (and we're not
    /// externally driven).
    pub(crate) fn is_expired(&self, now: f32) -> bool {
        if self.config.external_progress.is_some() {
            return false;
        }
        if self.created_at < 0.0 {
            return false;
        }
        (now - self.created_at) >= self.config.duration.as_secs_f32()
    }

    /// Progress 0.0..=1.0. External progress wins; otherwise time
    /// elapsed / duration.
    pub(crate) fn progress(&self, now: f32) -> f32 {
        if let Some(p) = self.config.external_progress {
            return p.clamp(0.0, 1.0);
        }
        if self.created_at < 0.0 {
            return 0.0;
        }
        let dur = self.config.duration.as_secs_f32();
        if dur <= 0.0 {
            return 1.0;
        }
        ((now - self.created_at) / dur).min(1.0)
    }
}

/// Toast notification manager.
///
/// Insert as a Bevy [`Resource`] (the [`crate::toast::ToastPlugin`]
/// adds a default one). Call `manager.toast(...)`, `.error(...)`, or
/// `.custom()` to enqueue a toast. The toast plugin's systems pick
/// the queue up each frame and spawn UI nodes.
#[derive(Resource, Debug)]
pub struct ToastManager {
    pub(crate) toasts: VecDeque<ToastRecord>,
    pub position: ToastPosition,
    pub max_toasts: usize,
    pub width: f32,
}

impl Default for ToastManager {
    fn default() -> Self {
        Self {
            toasts: VecDeque::new(),
            position: ToastPosition::default(),
            max_toasts: MAX_TOASTS,
            width: 356.0,
        }
    }
}

impl ToastManager {
    /// Enqueue a default-variant toast and return its [`ToastId`].
    pub fn toast(&mut self, message: impl Into<String>) -> ToastId {
        self.add(ToastConfig {
            id: 0,
            title: None,
            message: message.into(),
            variant: ToastVariant::Default,
            duration: Duration::from_secs_f32(DEFAULT_DURATION_SECS),
            dismissible: true,
            external_progress: None,
            show_progress: false,
        })
    }

    /// Enqueue a destructive (error) toast.
    pub fn error(&mut self, message: impl Into<String>) -> ToastId {
        self.add(ToastConfig {
            id: 0,
            title: None,
            message: message.into(),
            variant: ToastVariant::Destructive,
            duration: Duration::from_secs_f32(DEFAULT_DURATION_SECS),
            dismissible: true,
            external_progress: None,
            show_progress: false,
        })
    }

    /// Start a builder for a custom toast.
    pub fn custom(&mut self) -> ToastBuilder<'_> {
        ToastBuilder {
            manager: self,
            config: ToastConfig {
                id: 0,
                title: None,
                message: String::new(),
                variant: ToastVariant::Default,
                duration: Duration::from_secs_f32(DEFAULT_DURATION_SECS),
                dismissible: true,
                external_progress: None,
                show_progress: false,
            },
        }
    }

    pub(crate) fn add(&mut self, mut config: ToastConfig) -> ToastId {
        config.id = NEXT_ID.fetch_add(1, Ordering::Relaxed) + 1;
        let id = ToastId(config.id);
        self.toasts.push_back(ToastRecord::new(config));
        while self.toasts.len() > self.max_toasts {
            // Drop the oldest visible toast — render system will see
            // the missing record next frame and despawn its node.
            self.toasts.pop_front();
        }
        id
    }

    /// Update the progress bar of a toast (0.0..=1.0). While external
    /// progress is set, the toast won't auto-dismiss.
    pub fn set_progress(&mut self, id: ToastId, progress: f32) {
        if let Some(t) = self.find_mut(id) {
            t.config.external_progress = Some(progress.clamp(0.0, 1.0));
        }
    }

    /// Replace the message text on an existing toast in place.
    pub fn set_message(&mut self, id: ToastId, message: impl Into<String>) {
        if let Some(t) = self.find_mut(id) {
            t.config.message = message.into();
        }
    }

    /// Replace the title on an existing toast in place.
    pub fn set_title(&mut self, id: ToastId, title: impl Into<String>) {
        if let Some(t) = self.find_mut(id) {
            t.config.title = Some(title.into());
        }
    }

    /// Switch from external-progress mode back to the timed countdown,
    /// resetting the timer to "now". Useful at the end of a long task
    /// to show a brief "done" message before the toast fades.
    pub fn start_dismiss(&mut self, id: ToastId) {
        if let Some(t) = self.find_mut(id) {
            t.config.external_progress = None;
            // The render system re-stamps `created_at` when it
            // notices we want a fresh countdown.
            t.created_at = -1.0;
        }
    }

    /// Immediately remove a toast.
    pub fn dismiss(&mut self, id: ToastId) {
        self.toasts.retain(|t| t.config.id != id.0);
    }

    /// Drop every active toast.
    pub fn clear(&mut self) {
        self.toasts.clear();
    }

    fn find_mut(&mut self, id: ToastId) -> Option<&mut ToastRecord> {
        self.toasts.iter_mut().find(|t| t.config.id == id.0)
    }
}

/// Builder returned by [`ToastManager::custom`]. Chain configuration
/// then call `.show()` to enqueue.
pub struct ToastBuilder<'a> {
    manager: &'a mut ToastManager,
    config: ToastConfig,
}

impl<'a> ToastBuilder<'a> {
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.config.message = message.into();
        self
    }

    #[must_use]
    pub fn variant(mut self, variant: ToastVariant) -> Self {
        self.config.variant = variant;
        self
    }

    #[must_use]
    pub fn destructive(mut self) -> Self {
        self.config.variant = ToastVariant::Destructive;
        self
    }

    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.config.duration = duration;
        self
    }

    #[must_use]
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.config.dismissible = dismissible;
        self
    }

    /// Set initial external progress (0.0..=1.0). The toast won't
    /// auto-dismiss until you call
    /// [`ToastManager::start_dismiss`]. Implies
    /// [`Self::show_progress`].
    #[must_use]
    pub fn progress(mut self, progress: f32) -> Self {
        self.config.external_progress = Some(progress.clamp(0.0, 1.0));
        self.config.show_progress = true;
        self
    }

    /// Show the auto-dismiss countdown as a progress bar underneath
    /// the message. Off by default. External-progress toasts always
    /// show the bar regardless of this flag.
    #[must_use]
    pub fn show_progress(mut self, show: bool) -> Self {
        self.config.show_progress = show;
        self
    }

    pub fn show(self) -> ToastId {
        self.manager.add(self.config)
    }
}
