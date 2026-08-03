//! Menu surface tokens — shared by the context menu and the select trigger.
//!
//! # Why these are not in `tempera-theme`
//!
//! [`tempera_theme`] holds what *any* consumer of the design system reads: a
//! palette, a spacing scale, a type ramp. A dock reads a colour; a tree reads
//! a colour; neither will ever ask how tall a menu row is.
//!
//! These are widget geometry. They live here, next to the two widgets that
//! read them, for the same reason `ListRowTokens` lives next to the list row.
//!
//! # Why they are not in `context_menu` either
//!
//! Because [`crate::select`] reads them too. A select's trigger *is* a menu
//! surface — it opens one, and it takes the menu's hover colours so the two
//! match. Pushing these into `context_menu` would make `select` depend on
//! `context_menu` to know what colour to paint, which is a worse coupling
//! than one shared resource between siblings.
//!
//! Two readers is the whole justification. If `select` ever stops being a
//! menu surface, this belongs inside `context_menu`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use tempera_theme::{ColorPalette, FontHandle, Typography};

/// Sizing and colour for a popup menu surface.
///
/// A separate resource from the theme's buckets because these are tuned for
/// dense list rows, not for the generic input/button surface.
#[derive(Resource, Clone, Debug)]
pub struct MenuTokens {
    pub width: f32,
    pub item_height: f32,
    pub item_padding_x: f32,
    pub corner_radius: f32,
    pub border_width: f32,

    /// Hover/keyboard-focus row background.
    pub item_hover_bg: Color,
    /// Pressed row background.
    pub item_active_bg: Color,
    /// 1px line between item groups.
    pub separator: Color,
    /// Drop shadow color (alpha-encoded).
    pub shadow: Color,
}

impl Default for MenuTokens {
    fn default() -> Self {
        // Geometry is left as-is for now: `item_height` 26 and
        // `item_padding_x` 10 are off the scale (26 is not a multiple of any
        // scale member) and snapping them to it — 26 → 28, 10 → 8 — moves
        // pixels on screen. That is a deliberately separate, visible change.
        // `corner_radius` 6 *is* already the ×3/2 strand at step 1, and
        // `border_width` is a hairline, which answers to the display rather
        // than to the grid.
        Self {
            width: 220.0,
            item_height: 26.0,
            item_padding_x: 10.0,
            corner_radius: 6.0,
            border_width: 1.0,
            item_hover_bg: Color::srgba(1.0, 1.0, 1.0, 0.06),
            item_active_bg: Color::srgba(1.0, 1.0, 1.0, 0.10),
            separator: Color::srgba(1.0, 1.0, 1.0, 0.08),
            shadow: Color::srgba(0.0, 0.0, 0.0, 0.45),
        }
    }
}

/// Tokens read by the context-menu paint systems.
///
/// One of the `*Style` bundles every widget module declares — it names
/// exactly the tokens the widget reads, so adding a dependency means editing
/// the bundle and the coupling shows up at compile time.
#[derive(SystemParam)]
pub struct MenuStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub menu: Res<'w, MenuTokens>,
}

impl MenuStyle<'_> {
    /// Body-row text font (typography.sm).
    #[must_use]
    pub fn body_font(&self) -> TextFont {
        self.font.text_font(self.typography.sm)
    }

    /// Smaller shortcut-text font (typography.xs).
    #[must_use]
    pub fn shortcut_font(&self) -> TextFont {
        self.font.text_font(self.typography.xs)
    }
}
