//! Small components describing a button's appearance.
//!
//! Variant and size are independent: insert / remove either without
//! affecting the other. The paint system reads whichever is present
//! and falls back to [`ButtonVariant::Default`] / [`ButtonSize::Md`].

use bevy::prelude::*;

/// Tempera marker on `spawn_button` outputs. Differentiates a
/// tempera-styled button from any other entity that uses
/// [`bevy::ui_widgets::Button`] purely for click behavior (tab
/// triggers, dropdown triggers, etc.). The paint system filters on
/// this marker so we don't accidentally repaint every Button-bearing
/// entity in the app with the default primary-color fill.
#[derive(Component, Default, Debug)]
pub struct TemperaButton;

/// Opt-in tinting for an icon child. When present on a tempera
/// button, the paint system writes `ImageNode.color` on the button's
/// icon child between `resting` (idle) and `hover` (hovered/pressed).
/// Pair with [`crate::button::ButtonVariant::Ghost`] to get dawai's
/// toolbar-glyph pattern: no surface fill, icon recolors on hover.
///
/// Expects the icon to be rasterized as a tintable (typically white)
/// PNG so the `ImageNode.color` multiplication actually shows.
#[derive(Component, Clone, Copy, Debug)]
pub struct IconTint {
    pub resting: Color,
    pub hover: Color,
}

impl IconTint {
    pub const fn new(resting: Color, hover: Color) -> Self {
        Self { resting, hover }
    }
}

/// Visual variant of a button. Maps to a color recipe applied by the
/// paint system using the [`crate::ColorPalette`] resource.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Filled, primary-color background. The default if no variant is
    /// inserted on the button entity.
    #[default]
    Default,
    /// Filled, secondary (muted) background.
    Secondary,
    /// Transparent background, 1px border.
    Outline,
    /// Transparent background, no border — paints on hover only.
    Ghost,
    /// Text-only, underline on hover.
    Link,
    /// Filled with `destructive` color (delete / danger actions).
    Destructive,
}

/// Sizing preset for a button. Controls height, horizontal padding,
/// and text size. The `Icon*` variants are square with no horizontal
/// padding — pair them with [`super::ButtonContent::Icon`] for
/// shadcn-style icon-only buttons.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// 22px tall — tight chips / inline controls.
    Xs,
    /// 28px tall — compact dense toolbars.
    Sm,
    /// 32px tall — default for most buttons.
    #[default]
    Md,
    /// 40px tall — large CTAs.
    Lg,
    /// 32×32 square — shadcn `size="icon"`. Use with `Icon` content.
    Icon,
    /// 24×24 square — shadcn `size="icon-sm"`. Use with `Icon` content.
    IconSm,
}

impl ButtonSize {
    /// Total button height (and width, for icon sizes) in logical
    /// pixels.
    #[inline]
    #[must_use]
    pub const fn height(self) -> f32 {
        match self {
            Self::Xs => 22.0,
            Self::Sm => 28.0,
            Self::Md => 32.0,
            Self::Lg => 40.0,
            Self::Icon => 32.0,
            Self::IconSm => 24.0,
        }
    }

    /// Horizontal padding on each side. Icon sizes are square and
    /// zero-padded so the icon fills the widget.
    #[inline]
    #[must_use]
    pub const fn padding_x(self) -> f32 {
        match self {
            Self::Xs => 8.0,
            Self::Sm => 10.0,
            Self::Md => 14.0,
            Self::Lg => 18.0,
            Self::Icon | Self::IconSm => 0.0,
        }
    }

    /// True for the square icon-only sizes. The spawn helper sets an
    /// explicit `width = height` on these so flex doesn't size them
    /// from icon content alone.
    #[inline]
    #[must_use]
    pub const fn is_icon(self) -> bool {
        matches!(self, Self::Icon | Self::IconSm)
    }
}
