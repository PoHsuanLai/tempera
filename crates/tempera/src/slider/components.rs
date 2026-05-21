//! Small components describing slider visuals.

use bevy::prelude::*;

/// Marker on the inner "track" Node — the horizontal background bar.
/// The thumb's position is computed against this entity's measured
/// width.
#[derive(Component, Default, Debug)]
pub struct SliderTrack;

/// Marker on the filled portion of the track (from `range.start` to
/// the current value). Stretched per-frame by the paint system.
#[derive(Component, Default, Debug)]
pub struct SliderFill;

/// Sizing preset. Controls track height and thumb diameter.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliderSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl SliderSize {
    #[inline]
    #[must_use]
    pub const fn track_height(self) -> f32 {
        match self {
            Self::Sm => 4.0,
            Self::Md => 6.0,
            Self::Lg => 8.0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn thumb_diameter(self) -> f32 {
        match self {
            Self::Sm => 12.0,
            Self::Md => 16.0,
            Self::Lg => 20.0,
        }
    }

    /// Total row height (the slider's root Node) — leaves vertical
    /// room for the thumb to overflow the track.
    #[inline]
    #[must_use]
    pub const fn row_height(self) -> f32 {
        self.thumb_diameter() + 4.0
    }
}
