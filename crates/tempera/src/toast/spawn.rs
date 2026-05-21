use bevy::prelude::*;

use super::components::{
    ToastMessageText, ToastNode, ToastProgressFill, ToastTitleText, ToastVariant,
};
use super::manager::ToastRecord;
use super::systems::{
    variant_color, PROGRESS_HEIGHT_LOGICAL, TOAST_CORNER_RADIUS_LOGICAL, TOAST_PADDING_LOGICAL,
    TOAST_SPACING_LOGICAL, Z_TOAST,
};
use crate::theme::{ColorPalette, FontHandle, Typography};

/// Build the toast UI subtree:
/// ```
/// root (column, padded card)
///   ├── header row (icon + message + close-button placeholder)
///   │     ├── icon dot
///   │     └── text column (title? + message)
///   └── progress-bar row
///         └── fill (width: 0% initially, drive system updates each frame)
/// ```
/// Spawn the toast UI tree. Returns `(root, progress_fill)` — fill
/// is `Some` only when the toast was configured with a visible
/// progress bar (either explicit or external).
pub(crate) fn spawn_toast(
    commands: &mut Commands,
    record: &ToastRecord,
    width: f32,
    palette: &ColorPalette,
    typography: &Typography,
    font: &FontHandle,
) -> (Entity, Option<Entity>) {
    let accent = variant_color(palette, record.config.variant);
    let label_font = font.text_font(typography.sm);
    let title_font = font.text_font(typography.base);

    let root = commands
        .spawn((
            ToastNode { id: record.config.id },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(width),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(TOAST_PADDING_LOGICAL)),
                border_radius: BorderRadius::all(Val::Px(TOAST_CORNER_RADIUS_LOGICAL)),
                border: UiRect::all(Val::Px(1.0)),
                row_gap: Val::Px(TOAST_SPACING_LOGICAL),
                ..default()
            },
            BackgroundColor(palette.popover),
            BorderColor::all(palette.border),
            GlobalZIndex(Z_TOAST),
            bevy::picking::Pickable::IGNORE,
            Name::new("tempera::toast"),
        ))
        .id();

    // Header row — icon + text column.
    let header = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(TOAST_SPACING_LOGICAL),
                align_items: AlignItems::Start,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(root),
        ))
        .id();

    // Accent dot — stands in for the variant icon. (We don't ship
    // icon glyphs in tempera yet; this matches armas's visual weight.)
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

    if let Some(title) = &record.config.title {
        commands.spawn((
            ToastTitleText,
            Text::new(title.clone()),
            title_font,
            TextColor(palette.foreground),
            ChildOf(text_col),
        ));
    }
    commands.spawn((
        ToastMessageText,
        Text::new(record.config.message.clone()),
        label_font,
        TextColor(palette.muted_foreground),
        ChildOf(text_col),
    ));

    // Progress-bar track + fill — only when requested. Off by
    // default (matches shadcn Sonner); external-progress toasts force
    // it on via the builder.
    let needs_progress =
        record.config.show_progress || record.config.external_progress.is_some();
    let progress_fill = needs_progress.then(|| {
        let track = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(PROGRESS_HEIGHT_LOGICAL),
                    border_radius: BorderRadius::all(Val::Px(PROGRESS_HEIGHT_LOGICAL * 0.5)),
                    ..default()
                },
                BackgroundColor(palette.muted),
                ChildOf(root),
            ))
            .id();

        commands
            .spawn((
                ToastProgressFill,
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    border_radius: BorderRadius::all(Val::Px(PROGRESS_HEIGHT_LOGICAL * 0.5)),
                    ..default()
                },
                BackgroundColor(accent),
                ChildOf(track),
            ))
            .id()
    });

    // Suppress unused-variant warning when there's no other consumer.
    let _ = ToastVariant::Default;

    (root, progress_fill)
}
