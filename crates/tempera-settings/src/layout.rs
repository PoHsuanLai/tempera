//! Dialog metrics.

use bevy::prelude::*;

/// Size and spacing for the settings dialog.
///
/// A resource rather than constants so a host restyles without a fork.
///
/// Note what is **not** here: the title-bar height. The implementation this
/// replaces carried `const TITLE_BAR_HEIGHT: f32 = 44.0` and subtracted it
/// from the dialog height to size the content row — a hand-copy of a
/// tempera internal that nothing kept in step. If tempera changed its title
/// bar, the settings body would have silently overflowed or left a gap.
/// Here the content row is `flex_grow: 1.0` inside the space tempera hands
/// back, so the number never has to be known.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct SettingsLayout {
    /// Dialog card width, in logical pixels.
    pub width: f32,
    /// Dialog card height, in logical pixels.
    pub height: f32,
    /// Sidebar width. Fixed — the sidebar holds labels, not content.
    pub sidebar_width: f32,
    /// Logical pixels scrolled per wheel line.
    pub scroll_speed: f32,
}

impl Default for SettingsLayout {
    fn default() -> Self {
        Self {
            width: 720.0,
            height: 480.0,
            sidebar_width: 160.0,
            scroll_speed: 40.0,
        }
    }
}
