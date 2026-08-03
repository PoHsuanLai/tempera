//! Dialog — centered modal card on a translucent backdrop.
//!
//! Spawn a dialog tree once at startup with [`spawn_dialog`], parent
//! your widgets to the returned `content` entity, and toggle the root's
//! [`Visibility`] to show or hide it. The dialog fires
//! [`DialogDismissed`] when the user clicks the backdrop, clicks the
//! close button, or presses Escape — consumers react to this message
//! and flip visibility (and any app-state) themselves.
//!
//! The split keeps the dialog source-of-truth-agnostic: the "is this
//! open?" flag can live in whatever resource/component already drives
//! your app's state, instead of being hidden inside this module.
//!
//! ```ignore
//! use tempera::dialog::{spawn_dialog, DialogConfig, DialogStyle, DialogDismissed};
//!
//! fn build_settings_dialog(mut commands: Commands, style: DialogStyle) {
//!     let parts = spawn_dialog(
//!         &mut commands,
//!         &style,
//!         DialogConfig::new().title("Settings").size(720.0, 480.0),
//!     );
//!     commands.entity(parts.content).with_children(|p| {
//!         // ... your settings tabs/widgets ...
//!     });
//! }
//!
//! fn react(
//!     mut events: MessageReader<DialogDismissed>,
//!     mut q: Query<&mut Visibility, With<tempera::dialog::Dialog>>,
//! ) {
//!     for ev in events.read() {
//!         if let Ok(mut v) = q.get_mut(ev.dialog) {
//!             *v = Visibility::Hidden;
//!         }
//!     }
//! }
//! ```

use bevy::prelude::*;

use crate::theme::ThemePlugin;

mod components;
mod messages;
mod spawn;
mod systems;

pub use components::{Dialog, DialogBackdrop, DialogCard, DialogClose, DialogContent};
pub use messages::DialogDismissed;
pub use spawn::{
    CARD_HEIGHT, CARD_PADDING, CARD_RADIUS, CARD_WIDTH, DialogConfig, DialogParts, DialogStyle,
    TITLE_BAR_HEIGHT, TITLE_BAR_PADDING_X, spawn_dialog,
};

pub struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_message::<DialogDismissed>()
            .add_systems(Update, systems::dismiss_on_escape);
    }
}
