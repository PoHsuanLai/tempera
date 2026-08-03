use bevy::prelude::*;

/// Tempera marker on the switch root. Distinguishes a switch from a
/// regular [`bevy::ui_widgets::Checkbox`] visually — both share the
/// Checkbox behavior.
#[derive(Component, Default, Debug)]
pub struct Switch;

/// Marker on the thumb child. The paint system updates its `left`
/// based on the parent's `Checked` + [`SwitchSize`].
#[derive(Component, Default, Debug)]
pub struct SwitchThumb;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwitchSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl SwitchSize {
    #[inline]
    #[must_use]
    pub const fn track_width(self) -> f32 {
        match self {
            Self::Sm => 36.0,
            Self::Md => 44.0,
            Self::Lg => 52.0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn track_height(self) -> f32 {
        match self {
            Self::Sm => 20.0,
            Self::Md => 24.0,
            Self::Lg => 28.0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn thumb_diameter(self) -> f32 {
        self.track_height() - 2.0 * Self::INSET
    }

    /// Gap between the thumb and the track's edge.
    ///
    /// Lives here because two places need the same number and must agree:
    /// `spawn` places the thumb at rest, and `drive_switch` interpolates it
    /// each frame. They each declared their own `let inset = 2.0`, so a
    /// change to one would have made the thumb jump on its first animation —
    /// a bug that only shows up in motion, and only after the change that
    /// caused it.
    pub const INSET: f32 = 2.0;

    /// How far the thumb travels between off and on.
    ///
    /// The other quantity both sites derived independently.
    #[inline]
    #[must_use]
    pub const fn thumb_travel(self) -> f32 {
        self.track_width() - self.thumb_diameter() - 2.0 * Self::INSET
    }
}
