//! Theme tokens — composed from small, independent sub-resources.
//!
//! Each token bucket is its own [`Resource`] so widget systems only
//! declare the slice they actually read. A typography system doesn't
//! need to fight the color palette for scheduling slots, and the menu
//! sizing tokens don't have to ride along on every button repaint.
//!
//! ```ignore
//! // a widget system reads only what it needs:
//! fn paint_buttons(palette: Res<ColorPalette>, spacing: Res<Spacing>, ...) {}
//! ```
//!
//! For ergonomics, widgets that read several buckets at once should
//! expose a [`SystemParam`] (see e.g. [`MenuStyle`] below) so call sites
//! stay tidy.
//!
//! ## Defaults
//!
//! [`ThemePlugin`] installs dark-mode defaults (shadcn zinc). Override
//! any sub-resource at startup to recolor / resize / re-font that
//! aspect of every widget:
//!
//! ```ignore
//! app.add_plugins(ThemePlugin)
//!    .insert_resource(ColorPalette::light());
//! ```

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Installs the default token resources. Idempotent — `init_resource`
/// only runs if the resource is absent, so consumers can pre-insert a
/// custom palette/spacing/etc. before adding [`ThemePlugin`].
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ColorPalette>()
            .init_resource::<Spacing>()
            .init_resource::<Typography>()
            .init_resource::<FontHandle>()
            .init_resource::<MenuTokens>();
    }
}

// ---------------------------------------------------------------------------
// Font handle
// ---------------------------------------------------------------------------

/// Font handles used for widget text. Two slots: regular and bold.
///
/// `regular = None` falls back to Bevy's built-in default font
/// (covers ASCII; misses ⌘ / ⌫ / arrow glyphs). `bold = None` falls
/// back to `regular` (so widgets that ask for bold text on a regular-
/// only setup just get the regular weight, which is what every
/// existing tempera example does).
///
/// Bevy_text's `TextFont.weight` only takes effect on **variable**
/// font files. To support headline-weight emphasis with regular .otf
/// or .ttf bundles (e.g. Inter-Bold.otf), set `bold` explicitly. The
/// [`Self::text_font_bold`] helper produces a `TextFont` with the
/// bold handle when one is set; widgets that want bold conditionally
/// call this instead of [`Self::text_font`].
#[derive(Resource, Clone, Default, Debug)]
pub struct FontHandle {
    pub regular: Option<Handle<Font>>,
    pub bold: Option<Handle<Font>>,
}

impl FontHandle {
    /// New `FontHandle` with only the regular slot filled.
    #[must_use]
    pub fn regular(handle: Handle<Font>) -> Self {
        Self {
            regular: Some(handle),
            bold: None,
        }
    }

    /// New `FontHandle` with both slots filled.
    #[must_use]
    pub fn with_bold(handle: Handle<Font>, bold: Handle<Font>) -> Self {
        Self {
            regular: Some(handle),
            bold: Some(bold),
        }
    }

    /// Build a [`TextFont`] using the regular handle at the given size,
    /// falling back to Bevy's default font if no handle is set.
    #[must_use]
    pub fn text_font(&self, size: f32) -> TextFont {
        let mut tf = TextFont {
            font_size: FontSize::Px(size),
            ..default()
        };
        if let Some(h) = &self.regular {
            tf.font = FontSource::Handle(h.clone());
        }
        tf
    }

    /// Build a bold [`TextFont`] using the bold handle if set, otherwise
    /// the regular handle.
    #[must_use]
    pub fn text_font_bold(&self, size: f32) -> TextFont {
        let mut tf = TextFont {
            font_size: FontSize::Px(size),
            ..default()
        };
        if let Some(h) = &self.bold {
            tf.font = FontSource::Handle(h.clone());
        } else if let Some(h) = &self.regular {
            tf.font = FontSource::Handle(h.clone());
        }
        tf
    }
}

// ---------------------------------------------------------------------------
// Color palette — shadcn naming, bevy `Color`s.
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Debug)]
pub struct ColorPalette {
    // Surfaces
    pub background: Color,
    pub card: Color,
    pub popover: Color,

    // Foregrounds
    pub foreground: Color,
    pub card_foreground: Color,
    pub popover_foreground: Color,

    // Action colors
    pub primary: Color,
    pub primary_foreground: Color,
    pub secondary: Color,
    pub secondary_foreground: Color,
    pub muted: Color,
    pub muted_foreground: Color,
    pub accent: Color,
    pub accent_foreground: Color,
    pub destructive: Color,
    pub destructive_foreground: Color,

    // Lines
    pub border: Color,
    pub input: Color,
    pub ring: Color,

    // States
    pub hover: Color,
    pub focus: Color,
}

impl ColorPalette {
    /// Lift a base color toward white by a small fixed amount — the
    /// canonical "hover" brightening. Use this when a widget needs a
    /// hover tint that doesn't already exist in the palette as its own
    /// token.
    #[inline]
    #[must_use]
    pub fn hover_lift(base: Color, amount: f32) -> Color {
        let s = base.to_srgba();
        Color::srgba(
            (s.red + amount).clamp(0.0, 1.0),
            (s.green + amount).clamp(0.0, 1.0),
            (s.blue + amount).clamp(0.0, 1.0),
            s.alpha,
        )
    }

    /// shadcn zinc dark palette.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            background: rgb(9, 9, 11),
            card: rgb(9, 9, 11),
            popover: rgb(17, 17, 20),

            foreground: rgb(250, 250, 250),
            card_foreground: rgb(250, 250, 250),
            popover_foreground: rgb(238, 238, 243),

            primary: rgb(250, 250, 250),
            primary_foreground: rgb(24, 24, 27),
            // secondary / muted / accent follow shadcn v4 dark: muted
            // is mid-grey, accent is a notably lighter grey so
            // selected-vs-hovered toggle items contrast against each
            // other.
            secondary: rgb(64, 64, 67),
            secondary_foreground: rgb(250, 250, 250),
            muted: rgb(64, 64, 67),
            muted_foreground: rgb(161, 161, 170),
            accent: rgb(95, 95, 99),
            accent_foreground: rgb(250, 250, 250),
            destructive: rgb(245, 90, 90),
            destructive_foreground: rgb(250, 250, 250),

            border: Color::srgba(1.0, 1.0, 1.0, 0.08),
            input: rgb(82, 82, 91),
            ring: rgb(212, 212, 216),

            hover: rgb(39, 39, 42),
            focus: rgb(250, 250, 250),
        }
    }

    /// shadcn zinc light palette.
    #[must_use]
    pub fn light() -> Self {
        Self {
            background: rgb(255, 255, 255),
            card: rgb(255, 255, 255),
            popover: rgb(255, 255, 255),

            foreground: rgb(9, 9, 11),
            card_foreground: rgb(9, 9, 11),
            popover_foreground: rgb(9, 9, 11),

            primary: rgb(24, 24, 27),
            primary_foreground: rgb(250, 250, 250),
            secondary: rgb(244, 244, 245),
            secondary_foreground: rgb(24, 24, 27),
            muted: rgb(244, 244, 245),
            muted_foreground: rgb(113, 113, 122),
            accent: rgb(244, 244, 245),
            accent_foreground: rgb(24, 24, 27),
            destructive: rgb(239, 68, 68),
            destructive_foreground: rgb(250, 250, 250),

            border: rgb(228, 228, 231),
            input: rgb(228, 228, 231),
            ring: rgb(24, 24, 27),

            hover: rgb(244, 244, 245),
            focus: rgb(24, 24, 27),
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark()
    }
}

#[inline]
fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::srgb_u8(r, g, b)
}

// ---------------------------------------------------------------------------
// Spacing scale (xxs..xxl + corner radii) — matches shadcn/Tailwind.
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Debug)]
pub struct Spacing {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,

    pub corner_radius_micro: f32,
    pub corner_radius_tiny: f32,
    pub corner_radius_small: f32,
    pub corner_radius: f32,
    pub corner_radius_large: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            xxs: 2.0,
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            xxl: 48.0,
            corner_radius_micro: 2.0,
            corner_radius_tiny: 4.0,
            corner_radius_small: 6.0,
            corner_radius: 8.0,
            corner_radius_large: 12.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Typography scale (xxs..xxl).
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Debug)]
pub struct Typography {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub base: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for Typography {
    fn default() -> Self {
        // Values match the rendered pixel height egui produces with
        // these same numeric tokens, so tempera widgets look identical
        // to armas widgets when both share a Theme. Bevy_text and
        // egui both interpret `font_size` as logical pixels, but
        // armas-basic uses the slightly smaller scale (sm=12 vs
        // shadcn's 13) — we follow armas to keep dawai parity.
        Self {
            xxs: 8.0,
            xs: 10.0,
            sm: 12.0,
            base: 14.0,
            lg: 16.0,
            xl: 18.0,
            xxl: 24.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Menu-specific sizing tokens — separate resource because they're tuned
// for dense list rows, not the generic input/button surface.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// SystemParam bundles — curated read-only slices for widget systems.
//
// Each widget module declares a `*Style` bundle that names exactly the
// tokens it reads. Adding a new dependency means changing the bundle,
// which surfaces the coupling at compile time.
// ---------------------------------------------------------------------------

/// Tokens read by the context-menu paint systems.
#[derive(SystemParam)]
pub struct MenuStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub menu: Res<'w, MenuTokens>,
}

impl<'w> MenuStyle<'w> {
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
