//! Button widget — bevy_ui-based.
//!
//! Built on top of [`bevy::ui_widgets::Button`] (headless behavior: press
//! tracking, click + keyboard `Activate`, hover, focus, disabled).
//! Tempera adds the visuals — rounded rect, variant colors, optional
//! border, hover/press feedback, text and/or icon content.
//!
//! ## Composition
//!
//! Buttons are described by **small, independent components**, not a
//! single fat config struct:
//!
//! - [`Button`] (re-exported from bevy_ui_widgets) — the behavior marker
//! - [`ButtonVariant`] — visual style enum (Default / Secondary / Outline / Ghost / Link / Destructive)
//! - [`ButtonSize`] — sizing enum (Xs / Sm / Md / Lg)
//! - [`InteractionDisabled`] (Bevy built-in) — disabled state
//!
//! Insert / remove these independently to change a button at runtime.
//! Missing variant / size default to `Default` / `Md`.
//!
//! ## Spawning
//!
//! ```ignore
//! use tempera::button::{spawn_button, ButtonContent, ButtonVariant};
//!
//! let id = spawn_button(
//!     &mut commands,
//!     &style,
//!     ButtonContent::text("Save"),
//!     ButtonVariant::Default,
//! );
//! commands.entity(id).observe(|_: On<Activate>| info!("clicked!"));
//! ```

use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui_widgets::ButtonPlugin as BevyButtonPlugin;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use bevy::ui_widgets::{Activate, Button};
pub use components::{ButtonSize, ButtonVariant, IconTint, Selected, TemperaButton};
pub use spawn::{ButtonContent, ButtonStyle, spawn_button, spawn_button_sized};

pub struct ButtonStylePlugin;

impl Plugin for ButtonStylePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<InputDispatchPlugin>() {
            app.add_plugins(InputDispatchPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<BevyButtonPlugin>() {
            app.add_plugins(BevyButtonPlugin);
        }

        app.add_systems(
            Update,
            (systems::repaint_buttons, systems::repaint_icon_tints),
        );
    }
}
