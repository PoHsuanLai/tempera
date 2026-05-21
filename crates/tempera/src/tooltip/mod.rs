//! Tooltip — contextual help on hover.
//!
//! Add a [`Tooltip`] component to any UI node; on hover (after
//! `delay_ms`) tempera spawns a styled popup anchored to the node,
//! with a small arrow pointing back. Despawns on hover loss.
//!
//! Visuals follow shadcn's tooltip: inverted colors
//! (`bg-foreground`, `text-background`), 6px corner radius, 5px arrow.
//! Position auto-flips (Top → Bottom → Right → Left) if the preferred
//! side doesn't fit, matching armas.
//!
//! ## Composition
//!
//! - [`Tooltip`] — config on the target entity
//! - [`TooltipPopup`] — marker on the spawned popup; carries the
//!   target Entity so the sync system can re-anchor each frame
//! - [`TooltipArrow`] — marker on the arrow child
//!
//! ## Usage
//!
//! ```ignore
//! let button = spawn_button(&mut commands, &style, ButtonContent::text("Save"), ButtonVariant::Default);
//! commands.entity(button).insert(Tooltip::new("Save the project (⌘S)").delay(250));
//! ```

use bevy::prelude::*;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{Tooltip, TooltipArrow, TooltipPopup, TooltipPosition};

pub struct TooltipPlugin;

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(systems::on_hover_start);
        app.add_observer(systems::on_hover_end);
        app.add_systems(
            Update,
            (systems::open_tooltips, systems::sync_popup_positions).chain(),
        );
    }
}
