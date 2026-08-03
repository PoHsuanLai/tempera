//! Progress bar.
//!
//! ## Composition
//!
//! - [`Progress`] — root marker (the track)
//! - [`ProgressValue`] — current value in `0.0..=1.0`
//! - [`ProgressFill`] — child marker for the filled portion
//!
//! Mutate `ProgressValue` to update the bar; the paint system stretches
//! the fill child accordingly.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::theme::{ColorPalette, Metrics, Step, ThemePlugin};

#[derive(Component, Default, Debug)]
pub struct Progress;

/// Current value, clamped to `0.0..=1.0`.
#[derive(Component, Clone, Copy, Debug)]
pub struct ProgressValue(pub f32);

impl Default for ProgressValue {
    fn default() -> Self {
        Self(0.0)
    }
}

#[derive(Component, Default, Debug)]
pub struct ProgressFill;

#[derive(SystemParam)]
pub struct ProgressStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub metrics: Metrics<'w>,
}

/// Spawn a progress bar of `width` logical pixels, initially at `value`.
pub fn spawn_progress(
    commands: &mut Commands,
    style: &ProgressStyle,
    width: f32,
    value: f32,
) -> Entity {
    // Step 2 on the spacing scale — 8 at the default base.
    let height = style.metrics.gap(Step::new(2)).get();
    let id = commands
        .spawn((
            Progress,
            ProgressValue(value.clamp(0.0, 1.0)),
            Node {
                width: Val::Px(width),
                height: Val::Px(height),
                border_radius: BorderRadius::all(Val::Px(height * 0.5)),
                ..default()
            },
            BackgroundColor(style.palette.muted),
            Name::new("tempera::progress"),
        ))
        .id();

    commands.spawn((
        ProgressFill,
        Node {
            width: Val::Percent(value.clamp(0.0, 1.0) * 100.0),
            height: Val::Percent(100.0),
            border_radius: BorderRadius::all(Val::Px(height * 0.5)),
            ..default()
        },
        BackgroundColor(style.palette.primary),
        ChildOf(id),
    ));

    id
}

fn repaint_progress(
    bars: Query<(&ProgressValue, &Children), (With<Progress>, Changed<ProgressValue>)>,
    mut fills: Query<&mut Node, With<ProgressFill>>,
) {
    for (value, kids) in &bars {
        let pct = Val::Percent(value.0.clamp(0.0, 1.0) * 100.0);
        for child in kids.iter() {
            // Compared, not merely written: bevy_ui gates its taffy upload
            // on `Ref<Node>::is_changed()`, and a `DerefMut` sets that flag
            // whether or not the value moved. A bar re-told the same value
            // would otherwise re-upload on every telling.
            if let Ok(mut node) = fills.get_mut(child)
                && node.width != pct
            {
                node.width = pct;
            }
        }
    }
}

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_systems(Update, repaint_progress);
    }
}
