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
            // Placeholders only. `Default` cannot see a palette, and these
            // three are palette-dependent — `from_palette` below is where the
            // real values come from, and `sync_menu_tokens` keeps them there.
            // Left as the dark-theme figures so a consumer that never inserts
            // a palette still gets the appearance this crate always had.
            item_hover_bg: Color::srgba(1.0, 1.0, 1.0, 0.06),
            item_active_bg: Color::srgba(1.0, 1.0, 1.0, 0.10),
            separator: Color::srgba(1.0, 1.0, 1.0, 0.08),
        }
    }
}

impl MenuTokens {
    /// The row colours for a given palette, keeping the geometry as-is.
    ///
    /// The three were hardcoded white-alpha lifts — `srgba(1, 1, 1, 0.06)` and
    /// friends. On a dark surface that reads as a subtle highlight; on a light
    /// one it is white-on-white, so a hovered menu row and a select's own fill
    /// both vanished into the background.
    ///
    /// [`ColorPalette::step`] is the fix rather than a second pair of
    /// constants, because it moves *away* from the surface it is given: the
    /// same call lifts on dark and darkens on light, so there is one number
    /// per role instead of one per role per theme.
    pub fn from_palette(palette: &ColorPalette) -> Self {
        Self {
            item_hover_bg: ColorPalette::step(palette.popover, palette.popover, HOVER_LIFT),
            item_active_bg: ColorPalette::step(palette.popover, palette.popover, ACTIVE_LIFT),
            separator: palette.border,
            ..Self::default()
        }
    }
}

/// How far a hovered row moves from the menu surface.
const HOVER_LIFT: f32 = 0.06;

/// How far a pressed row moves. Enough to read as a second state next to
/// [`HOVER_LIFT`], which is what the two alphas it replaces encoded.
const ACTIVE_LIFT: f32 = 0.10;

/// Keep [`MenuTokens`] in step with the palette.
///
/// A resource rather than a component, so this cannot be a repaint system on
/// entities: the tokens are the *input* those systems read, and they are what
/// goes stale. Every consumer already re-reads them (`select` even names
/// `resource_changed::<MenuTokens>` in its run condition), so updating this
/// one resource repaints everything downstream for free.
pub fn sync_menu_tokens(palette: Res<ColorPalette>, mut tokens: ResMut<MenuTokens>) {
    if !palette.is_changed() {
        return;
    }
    let want = MenuTokens::from_palette(&palette);
    // Compared before writing: `MenuTokens` is read by several run conditions
    // via `resource_changed`, so a `DerefMut` that changed nothing would still
    // wake all of them.
    if tokens.item_hover_bg != want.item_hover_bg
        || tokens.item_active_bg != want.item_active_bg
        || tokens.separator != want.separator
    {
        tokens.item_hover_bg = want.item_hover_bg;
        tokens.item_active_bg = want.item_active_bg;
        tokens.separator = want.separator;
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
