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

/// Resize the fill to match the value.
///
/// Named for what it writes — `Node.width` — because the widget's *other*
/// system writes its colours, and two functions called `repaint_*` on one
/// widget is a coin toss over which one you are reading.
fn resize_progress_fill(
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

/// Repaint the track and the fill.
///
/// Separate from [`resize_progress_fill`] because the two answer to different
/// inputs: this runs when the *theme* moves, that one when the *value* does.
/// One system would need `Changed<ProgressValue> || palette_changed` and would
/// then recalculate a width on every theme swap and rewrite two colours on
/// every value tick — each doing the other's work for the other's trigger.
fn repaint_progress(
    palette: Res<crate::theme::ColorPalette>,
    mut tracks: Query<&mut BackgroundColor, (With<Progress>, Without<ProgressFill>)>,
    mut fills: Query<&mut BackgroundColor, With<ProgressFill>>,
) {
    for mut bg in &mut tracks {
        if bg.0 != palette.muted {
            bg.0 = palette.muted;
        }
    }
    for mut bg in &mut fills {
        if bg.0 != palette.primary {
            bg.0 = palette.primary;
        }
    }
}

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_systems(
            Update,
            (
                resize_progress_fill,
                repaint_progress.run_if(crate::theme::palette_changed),
            ),
        );
    }
}
