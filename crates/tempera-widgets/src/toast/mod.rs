//! Toast — non-modal notifications, entity-native.
//!
//! Each toast is an entity. Spawn with [`spawn`], [`spawn_error`], or
//! the [`ToastSpec`] builder, then mutate or despawn via the returned
//! [`Entity`]. The lifecycle systems pick up new entities each frame,
//! build their UI subtrees, advance the slide spring, and auto-dismiss
//! when the countdown elapses.
//!
//! ```ignore
//! use tempera::toast;
//!
//! fn save(mut commands: Commands) {
//!     // One-shot.
//!     toast::spawn(&mut commands, "Project saved");
//!
//!     // Destructive variant.
//!     toast::spawn_error(&mut commands, "Save failed");
//!
//!     // Progress toast — keep the Entity to update later.
//!     let e = toast::ToastSpec::new("Rendering audio…")
//!         .title("Exporting")
//!         .progress(0.0)
//!         .spawn(&mut commands);
//!
//!     // Later: mutate components directly.
//!     commands.entity(e)
//!         .insert(toast::ToastMessage("Halfway".into()))
//!         .insert(toast::ToastExternalProgress(0.5));
//! }
//! ```
//!
//! Toasts slide in from the configured corner (default
//! [`ToastPosition::BottomRight`], matching shadcn's Sonner), show a
//! variant-colored accent dot, and either auto-dismiss after their
//! [`ToastDuration`] (with a countdown progress bar) or follow an
//! external progress value supplied via [`ToastExternalProgress`].

use bevy::prelude::*;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{
    Toast, ToastCreated, ToastDismissible, ToastDuration, ToastExternalProgress, ToastMessage,
    ToastNodes, ToastPosition, ToastShowProgress, ToastSlide, ToastTitle, ToastVariant,
};
pub use spawn::{spawn, spawn_error, ToastSpec};

/// Toast-stack configuration. Mutate to change where toasts anchor,
/// how wide they render, or the max-on-screen ceiling.
#[derive(Resource, Clone, Debug)]
pub struct ToastConfig {
    pub position: ToastPosition,
    pub width: f32,
    pub max_toasts: usize,
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self {
            position: ToastPosition::default(),
            width: 356.0,
            max_toasts: 5,
        }
    }
}

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.init_resource::<ToastConfig>().add_systems(
            Update,
            (systems::reconcile_toast_ui, systems::tick_toasts).chain(),
        );
    }
}
