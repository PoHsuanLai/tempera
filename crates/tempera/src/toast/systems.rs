use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::HashSet;

use super::components::{
    ToastMessageText, ToastNode, ToastPosition, ToastProgressFill, ToastTitleText, ToastVariant,
};
use super::manager::ToastManager;
use crate::anim::Spring;
use crate::theme::{ColorPalette, FontHandle, Typography};

const TOAST_PADDING: f32 = 16.0;
const TOAST_CORNER_RADIUS: f32 = 8.0;
const TOAST_HEIGHT: f32 = 70.0;
const TOAST_SPACING: f32 = 8.0;
const TOAST_MARGIN: f32 = 16.0;
const PROGRESS_HEIGHT: f32 = 2.0;
/// Slide offset in logical pixels at slide=0.0 (toast fully off-edge).
const SLIDE_OFFSET: f32 = 50.0;
/// Z-order so toasts paint above tooltips and menus.
pub(crate) const Z_TOAST: i32 = 3000;

/// Spawn UI nodes for newly-added toasts, despawn nodes for dismissed
/// ones, and re-anchor the stack each frame so toasts slide-in from
/// the chosen corner.
pub(crate) fn drive_toasts(
    time: Res<Time>,
    palette: Res<ColorPalette>,
    typography: Res<Typography>,
    font: Res<FontHandle>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut manager: ResMut<ToastManager>,
    existing: Query<(Entity, &ToastNode)>,
    mut nodes: Query<&mut Node, (With<ToastNode>, Without<ToastProgressFill>)>,
    mut message_texts: Query<
        (&ChildOf, &mut Text),
        (With<ToastMessageText>, Without<ToastTitleText>),
    >,
    mut fill_nodes: Query<&mut Node, (With<ToastProgressFill>, Without<ToastNode>)>,
    mut commands: Commands,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());
    let now = time.elapsed_secs();
    let dt = time.delta_secs();

    // 1. Drop expired (timed) toasts from the queue.
    manager.toasts.retain(|t| !t.is_expired(now));

    // 2. Build a set of active toast ids so we can detect stale nodes.
    let active_ids: HashSet<u64> = manager.toasts.iter().map(|t| t.config.id).collect();

    // 3. Despawn nodes whose record is gone.
    for (entity, marker) in &existing {
        if !active_ids.contains(&marker.id) {
            commands.entity(entity).try_despawn();
        }
    }

    // 4. For each active toast, ensure a node exists and is anchored.
    //    Iterate newest → oldest so the newest sits closest to the
    //    corner (matches shadcn Sonner).
    let position = manager.position;
    let width = manager.width;
    let total = manager.toasts.len();
    for (record_index, record) in manager.toasts.iter_mut().enumerate() {
        let index = total - 1 - record_index;
        if record.created_at < 0.0 {
            record.created_at = now;
        }

        // Advance slide spring toward 1.0 (fully on-screen).
        let mut spring = Spring::new(record.slide, 1.0).params(250.0, 25.0);
        spring.velocity = record.slide_velocity;
        spring.update(dt);
        record.slide = spring.value;
        record.slide_velocity = spring.velocity;

        // Update message text in place if we already have a node.
        if let Some(entity) = record.node {
            for (parent, mut text) in &mut message_texts {
                if parent.0 == entity {
                    if text.0 != record.config.message {
                        text.0 = record.config.message.clone();
                    }
                    break;
                }
            }
        }

        // Spawn the node on first sighting.
        if record.node.is_none() {
            let (entity, fill) = super::spawn::spawn_toast(
                &mut commands,
                record,
                width,
                &palette,
                &typography,
                &font,
            );
            record.node = Some(entity);
            record.progress_fill = fill;
        }

        // Anchor + slide offset.
        let stack_offset = (TOAST_HEIGHT + TOAST_SPACING) * index as f32;
        let slide_t = record.slide.clamp(0.0, 1.0);
        let (left, right, top, bottom) =
            anchor_for(position, window_size, stack_offset, slide_t, width);

        if let Some(entity) = record.node
            && let Ok(mut node) = nodes.get_mut(entity)
        {
            node.left = left;
            node.right = right;
            node.top = top;
            node.bottom = bottom;
        }

        // Update progress-bar fill width — direct entity lookup.
        if let Some(fill) = record.progress_fill
            && let Ok(mut node) = fill_nodes.get_mut(fill)
        {
            node.width = Val::Percent(record.progress(now) * 100.0);
        }
    }
}

/// Anchor + slide-offset for a toast at `index` in the stack.
/// Returns (left, right, top, bottom) in `Val`s — exactly one of each
/// axis is `Auto` so flex anchors from the chosen edge.
fn anchor_for(
    position: ToastPosition,
    window: Vec2,
    stack: f32,
    slide_t: f32,
    _width: f32,
) -> (Val, Val, Val, Val) {
    let slide_in = SLIDE_OFFSET * (1.0 - slide_t);
    let v_offset = TOAST_MARGIN + stack;
    match position {
        ToastPosition::TopLeft => (
            Val::Px(TOAST_MARGIN - slide_in),
            Val::Auto,
            Val::Px(v_offset),
            Val::Auto,
        ),
        ToastPosition::TopCenter => {
            // For center, position via `left` + `right` matching; the
            // popup's own intrinsic width sits centered. We approximate
            // by computing midpoint.
            let mid_left = (window.x * 0.5) - 178.0; // ~width/2 default
            (
                Val::Px(mid_left),
                Val::Auto,
                Val::Px(v_offset - slide_in.signum() * 0.0),
                Val::Auto,
            )
        }
        ToastPosition::TopRight => (
            Val::Auto,
            Val::Px(TOAST_MARGIN - slide_in),
            Val::Px(v_offset),
            Val::Auto,
        ),
        ToastPosition::BottomLeft => (
            Val::Px(TOAST_MARGIN - slide_in),
            Val::Auto,
            Val::Auto,
            Val::Px(v_offset),
        ),
        ToastPosition::BottomCenter => {
            let mid_left = (window.x * 0.5) - 178.0;
            (
                Val::Px(mid_left),
                Val::Auto,
                Val::Auto,
                Val::Px(v_offset),
            )
        }
        ToastPosition::BottomRight => (
            Val::Auto,
            Val::Px(TOAST_MARGIN - slide_in),
            Val::Auto,
            Val::Px(v_offset),
        ),
    }
}

/// Helper used by the spawn module — exposed as `pub(super)` so the
/// systems module can reach in without a circular import.
#[inline]
pub(super) fn variant_color(palette: &ColorPalette, variant: ToastVariant) -> Color {
    match variant {
        ToastVariant::Default => palette.foreground,
        ToastVariant::Destructive => palette.destructive,
    }
}

pub(super) const TOAST_PADDING_LOGICAL: f32 = TOAST_PADDING;
pub(super) const TOAST_CORNER_RADIUS_LOGICAL: f32 = TOAST_CORNER_RADIUS;
pub(super) const TOAST_SPACING_LOGICAL: f32 = TOAST_SPACING;
pub(super) const PROGRESS_HEIGHT_LOGICAL: f32 = PROGRESS_HEIGHT;
