//! The plugin.

use bevy::prelude::*;

use crate::build::{SettingsBuildSet, SettingsCloseIcon, SettingsTitle, build_settings};
use crate::layout::SettingsLayout;
use crate::systems::{
    apply_active_tab, repaint_sidebar, repaint_sidebar_hover, repaint_sidebar_surface, scroll_body,
    sync_visibility,
};

/// A tabbed settings dialog.
///
/// Declare tabs by spawning them; this plugin builds a sidebar entry and a
/// body for each, and shows one at a time.
///
/// ```ignore
/// app.add_plugins(TemperaSettingsPlugin)
///     .insert_resource(SettingsTitle("Preferences".into()));
///
/// commands.spawn((SettingsTab, TabId::from("general"), TabLabel::from("General"), TabOrder(10)));
/// commands.spawn((SettingsTab, TabId::from("audio"), TabLabel::from("Audio"), TabOrder(20)));
/// ```
pub struct TemperaSettingsPlugin;

impl Plugin for TemperaSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsLayout>()
            .init_resource::<SettingsTitle>()
            .init_resource::<SettingsCloseIcon>();

        // The build runs in `Update` rather than `Startup` because it is
        // also the reconcile: a tab an extension declares on frame 900 has
        // to produce the same sidebar entry as one declared on frame 1. A
        // separate startup path would be a second implementation to keep in
        // step.
        app.add_systems(Update, build_settings.in_set(SettingsBuildSet))
            .add_systems(
                Update,
                (
                    apply_active_tab,
                    repaint_sidebar,
                    repaint_sidebar_hover,
                    // Only on the frames the theme moved — the surface has no
                    // per-entity trigger of its own.
                    repaint_sidebar_surface.run_if(tempera::theme::palette_changed),
                    sync_visibility,
                    scroll_body,
                )
                    .after(SettingsBuildSet),
            );
    }
}
