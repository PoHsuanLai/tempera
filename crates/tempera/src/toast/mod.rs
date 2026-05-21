//! Toast — non-modal notifications.
//!
//! Insert a [`ToastManager`] resource (the [`ToastPlugin`] does this
//! for you), then call its methods from anywhere:
//!
//! ```ignore
//! fn save(mut toasts: ResMut<ToastManager>) {
//!     toasts.toast("Project saved");
//!     // Or with a builder:
//!     toasts.custom()
//!         .title("Exporting")
//!         .message("Rendering audio…")
//!         .progress(0.0)  // externally-driven; won't auto-dismiss
//!         .show();
//! }
//! ```
//!
//! Toasts slide in from the configured corner (default
//! [`ToastPosition::BottomRight`], matching shadcn's Sonner), show a
//! variant-colored accent dot, and either auto-dismiss after their
//! duration (with a countdown progress bar) or follow an external
//! progress value.

use bevy::prelude::*;

use crate::theme::ThemePlugin;

mod components;
mod manager;
mod spawn;
mod systems;

pub use components::{ToastId, ToastNode, ToastPosition, ToastVariant};
pub use manager::{ToastBuilder, ToastManager};

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.init_resource::<ToastManager>();
        app.add_systems(Update, systems::drive_toasts);
    }
}
