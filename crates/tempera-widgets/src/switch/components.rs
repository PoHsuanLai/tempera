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
        self.track_height() - 4.0
    }
}
