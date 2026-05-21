//! Separator — a 1px line in the theme's `border` color.
//!
//! Inserts no behavior. The whole widget is a styled Node.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::theme::{ColorPalette, ThemePlugin};

#[derive(SystemParam)]
pub struct SeparatorStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
}

/// Orientation of a separator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SeparatorAxis {
    #[default]
    Horizontal,
    Vertical,
}

/// Spawn a separator. `length` is the long edge in logical pixels;
/// the short edge is 1px. Pass `None` for `length` to stretch to the
/// parent (100%).
pub fn spawn_separator(
    commands: &mut Commands,
    style: &SeparatorStyle,
    axis: SeparatorAxis,
    length: Option<f32>,
) -> Entity {
    let (w, h) = match axis {
        SeparatorAxis::Horizontal => (
            length.map_or(Val::Percent(100.0), Val::Px),
            Val::Px(1.0),
        ),
        SeparatorAxis::Vertical => (
            Val::Px(1.0),
            length.map_or(Val::Percent(100.0), Val::Px),
        ),
    };
    commands
        .spawn((
            Node {
                width: w,
                height: h,
                ..default()
            },
            BackgroundColor(style.palette.border),
            Name::new("tempera::separator"),
        ))
        .id()
}

pub struct SeparatorPlugin;

impl Plugin for SeparatorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
    }
}
