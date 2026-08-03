//! Toast lifecycle + UI reconciliation.
//!
//! Two systems:
//!
//! - [`reconcile_toast_ui`] — for any toast entity that doesn't yet
//!   have [`ToastNodes`], spawn the UI subtree (root + header + text +
//!   optional progress bar) and attach the handle component.
//! - [`tick_toasts`] — every frame:
//!     - stamp [`ToastCreated`] the first time we see a toast,
//!     - advance the slide spring,
//!     - re-anchor the stack (newest closest to the corner),
//!     - update message text on `Changed<ToastMessage>`,
//!     - update the progress-bar fill width,
//!     - despawn expired toasts (countdown finished, no external
//!       progress component) plus their UI subtree.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::ToastConfig;
use super::components::{
    Toast, ToastCreated, ToastDuration, ToastExternalProgress, ToastMessage, ToastNodes,
    ToastPosition, ToastShowProgress, ToastSlide, ToastTitle, ToastVariant,
};
use crate::anim::Spring;
use crate::theme::{ColorPalette, FontHandle, Step, Tokens, Typography};

/// The toast's own geometry, resolved from the spacing scale.
///
/// Five values used to sit here as literals — 16, 8, 8, 16, 2 — every one of
/// them a member of the scale written out by hand. They are steps 4, 2, 2, 4
/// and −2, so a change to the base moves them now.
struct ToastMetrics {
    padding: f32,
    corner_radius: f32,
    spacing: f32,
    margin: f32,
    progress_height: f32,
}

impl ToastMetrics {
    fn from(tokens: &Tokens) -> Self {
        let at = |n: i8| tokens.scale.at(Step::new(n)).get();
        Self {
            padding: at(4),
            corner_radius: at(2),
            spacing: at(2),
            margin: at(4),
            progress_height: at(-2),
        }
    }
}

/// Assumed toast height for stacking, in logical pixels.
///
/// **Off the scale deliberately, and not a control height either.** A toast is
/// sized by its own content — a title, a wrapped message, an optional progress
/// bar — and this is the figure the stack offset assumes before any of that has
/// been laid out. Snapping it to the grid would make the stack spacing wrong
/// rather than making it principled.
const TOAST_HEIGHT: f32 = 70.0;

/// Slide offset in logical pixels at slide=0.0 (toast fully off-edge).
///
/// An animation distance, not layout: how far off-screen the toast starts.
/// Nothing aligns to it, so it answers to how the motion reads rather than to
/// the grid.
const SLIDE_OFFSET: f32 = 50.0;
/// Z-order so toasts paint above tooltips and menus.
const Z_TOAST: i32 = 3000;

/// Build a UI subtree for any toast entity that doesn't have one yet,
/// and attach [`ToastNodes`] so the tick system can find the nodes
/// without re-querying.
pub(crate) fn reconcile_toast_ui(
    mut commands: Commands,
    palette: Res<ColorPalette>,
    typography: Res<Typography>,
    font: Res<FontHandle>,
    tokens: Res<Tokens>,
    config: Res<ToastConfig>,
    toasts: Query<
        (
            Entity,
            &ToastVariant,
            &ToastMessage,
            Option<&ToastTitle>,
            Option<&ToastExternalProgress>,
            Has<ToastShowProgress>,
        ),
        (With<Toast>, Without<ToastNodes>),
    >,
) {
    let metrics = ToastMetrics::from(&tokens);
    for (toast, variant, message, title, external_progress, show_progress) in &toasts {
        let nodes = spawn_subtree(
            &mut commands,
            toast,
            *variant,
            message,
            title,
            external_progress.is_some() || show_progress,
            config.width,
            &palette,
            &typography,
            &font,
            &metrics,
        );
        commands.entity(toast).insert(nodes);
    }
}

/// Drive every active toast each frame.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick_toasts(
    mut commands: Commands,
    time: Res<Time>,
    tokens: Res<Tokens>,
    config: Res<ToastConfig>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut toasts: Query<
        (
            Entity,
            &ToastNodes,
            &ToastDuration,
            Option<&mut ToastCreated>,
            &mut ToastSlide,
            &ToastMessage,
            Option<&ToastExternalProgress>,
        ),
        With<Toast>,
    >,
    changed_messages: Query<(&ToastNodes, &ToastMessage), Changed<ToastMessage>>,
    mut node_q: Query<&mut Node>,
    mut text_q: Query<&mut Text>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let window_size = Vec2::new(window.width(), window.height());
    let metrics = ToastMetrics::from(&tokens);
    let now = time.elapsed_secs();
    let dt = time.delta_secs();

    // 1. Update message text in place for any toast whose message changed.
    for (nodes, message) in &changed_messages {
        if let Ok(mut text) = text_q.get_mut(nodes.message_text) {
            if text.0 != message.0 {
                text.0 = message.0.clone();
            }
        }
    }

    // 2. Snapshot every active toast's spawn order so we can re-anchor
    //    the stack with newest closest to the corner.
    let mut entries: Vec<(Entity, f32)> = toasts
        .iter()
        .filter_map(|(e, _, _, created, _, _, _)| created.map(|c| (e, c.0)))
        .collect();
    // Toasts that haven't been stamped yet (this is their first frame)
    // get the current timestamp here so they participate in the stack
    // ordering on the same frame they appear.
    for (e, _, _, created, _, _, _) in &toasts {
        if created.is_none() {
            entries.push((e, now));
        }
    }
    entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let total = entries.len();

    // 3. Drive each toast.
    let mut to_despawn: Vec<Entity> = Vec::new();
    for (entity, nodes, duration, mut created, mut slide, _message, external) in &mut toasts {
        // Stamp creation time on first sighting.
        let created_at = match created.as_mut() {
            Some(c) => c.0,
            None => {
                commands.entity(entity).insert(ToastCreated(now));
                now
            }
        };

        // Expire check (only when no external progress is driving it).
        if external.is_none() {
            let elapsed = now - created_at;
            if elapsed >= duration.0.as_secs_f32() {
                to_despawn.push(entity);
                continue;
            }
        }

        // Advance slide spring toward 1.0.
        let mut spring = Spring::new(slide.value, 1.0).params(250.0, 25.0);
        spring.velocity = slide.velocity;
        spring.update(dt);
        slide.value = spring.value;
        slide.velocity = spring.velocity;

        // Stack index (oldest = bottom of stack, near the corner = newest).
        let stack_index = entries
            .iter()
            .rposition(|(e, _)| *e == entity)
            .map(|pos| total - 1 - pos)
            .unwrap_or(0);
        let stack_offset = (TOAST_HEIGHT + metrics.spacing) * stack_index as f32;
        let slide_t = slide.value.clamp(0.0, 1.0);
        let (left, right, top, bottom) = anchor_for(
            config.position,
            window_size,
            stack_offset,
            slide_t,
            metrics.margin,
        );
        if let Ok(mut node) = node_q.get_mut(nodes.root) {
            node.left = left;
            node.right = right;
            node.top = top;
            node.bottom = bottom;
        }

        // Progress-bar fill.
        if let Some(fill) = nodes.progress_fill
            && let Ok(mut node) = node_q.get_mut(fill)
        {
            let p = match external {
                Some(ext) => ext.0.clamp(0.0, 1.0),
                None => {
                    let dur = duration.0.as_secs_f32();
                    if dur <= 0.0 {
                        1.0
                    } else {
                        ((now - created_at) / dur).clamp(0.0, 1.0)
                    }
                }
            };
            node.width = Val::Percent(p * 100.0);
        }
    }

    // 4. Enforce max_toasts. Despawn the oldest above the ceiling.
    let max = config.max_toasts;
    if entries.len() > max {
        let excess = entries.len() - max;
        for (entity, _) in entries.iter().take(excess) {
            if !to_despawn.contains(entity) {
                to_despawn.push(*entity);
            }
        }
    }

    for entity in to_despawn {
        // Despawn the UI subtree by recursively dropping the root, then
        // drop the toast data entity itself.
        if let Ok((_, nodes, _, _, _, _, _)) = toasts.get(entity) {
            commands.entity(nodes.root).despawn();
        }
        commands.entity(entity).despawn();
    }
}

/// Anchor + slide-offset for a toast at `stack_index` in the stack.
/// Returns (left, right, top, bottom) in `Val`s — exactly one of each
/// axis is `Auto` so flex anchors from the chosen edge.
fn anchor_for(
    position: ToastPosition,
    window: Vec2,
    stack: f32,
    slide_t: f32,
    margin: f32,
) -> (Val, Val, Val, Val) {
    let slide_in = SLIDE_OFFSET * (1.0 - slide_t);
    let v_offset = margin + stack;
    match position {
        ToastPosition::TopLeft => (
            Val::Px(margin - slide_in),
            Val::Auto,
            Val::Px(v_offset),
            Val::Auto,
        ),
        ToastPosition::TopCenter => {
            let mid_left = (window.x * 0.5) - 178.0;
            (Val::Px(mid_left), Val::Auto, Val::Px(v_offset), Val::Auto)
        }
        ToastPosition::TopRight => (
            Val::Auto,
            Val::Px(margin - slide_in),
            Val::Px(v_offset),
            Val::Auto,
        ),
        ToastPosition::BottomLeft => (
            Val::Px(margin - slide_in),
            Val::Auto,
            Val::Auto,
            Val::Px(v_offset),
        ),
        ToastPosition::BottomCenter => {
            let mid_left = (window.x * 0.5) - 178.0;
            (Val::Px(mid_left), Val::Auto, Val::Auto, Val::Px(v_offset))
        }
        ToastPosition::BottomRight => (
            Val::Auto,
            Val::Px(margin - slide_in),
            Val::Auto,
            Val::Px(v_offset),
        ),
    }
}

/// Spawn the UI tree:
/// ```text
/// root (column, padded card)
///   ├── header row (icon + text column)
///   │     ├── accent dot
///   │     └── text column (title? + message)
///   └── progress-bar track (optional)
///         └── fill (width updated each frame)
/// ```
#[allow(clippy::too_many_arguments)]
fn spawn_subtree(
    commands: &mut Commands,
    toast_entity: Entity,
    variant: ToastVariant,
    message: &ToastMessage,
    title: Option<&ToastTitle>,
    show_progress: bool,
    width: f32,
    palette: &ColorPalette,
    typography: &Typography,
    font: &FontHandle,
    metrics: &ToastMetrics,
) -> ToastNodes {
    let accent = variant_color(palette, variant);
    let label_font = font.text_font(typography.sm);
    let title_font = font.text_font(typography.base);

    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(width),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(metrics.padding)),
                border_radius: BorderRadius::all(Val::Px(metrics.corner_radius)),
                border: UiRect::all(Val::Px(1.0)),
                row_gap: Val::Px(metrics.spacing),
                ..default()
            },
            BackgroundColor(palette.popover),
            BorderColor::all(palette.border),
            GlobalZIndex(Z_TOAST),
            bevy::picking::Pickable::IGNORE,
            Name::new(format!("tempera::toast::ui({toast_entity:?})")),
        ))
        .id();

    // Header row — accent dot + text column.
    let header = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(metrics.spacing),
                align_items: AlignItems::Start,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(root),
        ))
        .id();

    commands.spawn((
        Node {
            width: Val::Px(8.0),
            height: Val::Px(8.0),
            margin: UiRect::top(Val::Px(6.0)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(accent),
        ChildOf(header),
    ));

    let text_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                flex_grow: 1.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(header),
        ))
        .id();

    let title_text = title.map(|t| {
        commands
            .spawn((
                Text::new(t.0.clone()),
                title_font,
                TextColor(palette.foreground),
                ChildOf(text_col),
            ))
            .id()
    });

    let message_text = commands
        .spawn((
            Text::new(message.0.clone()),
            label_font,
            TextColor(palette.muted_foreground),
            ChildOf(text_col),
        ))
        .id();

    let progress_fill = show_progress.then(|| {
        let track = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(metrics.progress_height),
                    border_radius: BorderRadius::all(Val::Px(metrics.progress_height * 0.5)),
                    ..default()
                },
                BackgroundColor(palette.muted),
                ChildOf(root),
            ))
            .id();

        commands
            .spawn((
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    border_radius: BorderRadius::all(Val::Px(metrics.progress_height * 0.5)),
                    ..default()
                },
                BackgroundColor(accent),
                ChildOf(track),
            ))
            .id()
    });

    ToastNodes {
        root,
        message_text,
        title_text,
        progress_fill,
    }
}

#[inline]
fn variant_color(palette: &ColorPalette, variant: ToastVariant) -> Color {
    match variant {
        ToastVariant::Default => palette.foreground,
        ToastVariant::Destructive => palette.destructive,
    }
}
