//! Slider widget — bevy_ui-based.
//!
//! Built on top of [`bevy::ui_widgets::Slider`] (headless behavior:
//! drag, arrow-key step, snap / drag / step track-click modes, value
//! events). Tempera adds:
//!
//! 1. **Visual structure** — track + filled portion + thumb child marked
//!    with [`SliderThumb`].
//! 2. **Style-sync** — moves the thumb's `left` and resizes the fill
//!    based on the current [`SliderValue`].
//! 3. **`slider_self_update`** — installed automatically, so the slider
//!    mutates its own [`SliderValue`] on drag/keyboard. Override by
//!    observing [`ValueChange<f32>`] yourself and writing the value
//!    elsewhere.
//!
//! ## Composition
//!
//! - [`Slider`] (re-exported from bevy_ui_widgets) — behavior + drag
//! - [`SliderValue`] / [`SliderRange`] / [`SliderStep`] — value state
//! - [`SliderSize`] — visual sizing preset (Sm / Md / Lg)
//! - [`InteractionDisabled`] (Bevy built-in) — disabled state
//!
//! ## Spawning
//!
//! ```ignore
//! use tempera::slider::{spawn_slider, SliderSize};
//!
//! let id = spawn_slider(
//!     &mut commands,
//!     &style,
//!     SliderRange::new(0.0, 100.0),
//!     SliderValue(50.0),
//! );
//! commands.entity(id).observe(|on: On<ValueChange<f32>>| {
//!     info!("new value: {}", on.value);
//! });
//! ```

use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui_widgets::{slider_self_update, SliderPlugin as BevySliderPlugin};

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use bevy::ui_widgets::{
    Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick, ValueChange,
};
pub use components::{SliderFill, SliderSize, SliderTrack};
pub use spawn::{spawn_slider, SliderStyle};

pub struct SliderStylePlugin;

impl Plugin for SliderStylePlugin {
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
        if !app.is_plugin_added::<BevySliderPlugin>() {
            app.add_plugins(BevySliderPlugin);
        }

        // Bevy's SliderPlugin does NOT install `slider_self_update` by
        // default (the headless slider lets you choose internal vs
        // external state). Tempera defaults to self-updating so callers
        // get a working slider out of the box.
        app.add_observer(slider_self_update);

        app.add_systems(Update, (systems::reposition_thumb, systems::repaint_slider));
    }
}
