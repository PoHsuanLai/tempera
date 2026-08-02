use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::{
    ListRow, ListRowBadge, ListRowId, ListRowLead, ListRowMeta, ListRowSubtitle, ListRowTitle,
    ListRowTrail,
};
use crate::theme::{ColorPalette, FontHandle, Spacing, Typography};

/// Row metrics.
///
/// A resource rather than constants so an app restyles every list at once —
/// and so the trailing slot's floor is written in exactly one place. The
/// code this widget replaces had `Val::Px(200.0)` hand-written at eight
/// call sites beside a `CONTROL_WIDTH` constant that already said 200.
#[derive(Resource, Clone, Debug)]
pub struct ListRowTokens {
    /// Horizontal padding inside the row.
    pub padding_x: f32,
    /// Vertical padding inside the row.
    pub padding_y: f32,
    /// Gap below each row.
    pub row_gap: f32,
    /// Gap between the leading column and the trailing slot.
    pub column_gap: f32,
    /// Floor for the trailing slot. It grows past this to fit its content.
    pub trail_min_width: f32,
    /// Corner rounding on the hover fill.
    pub corner_radius: f32,
    /// Hard character cap on the subtitle, `None` to rely on flex alone.
    ///
    /// A cap exists because a description can be arbitrarily long and flex
    /// shrink alone resolves to an overlap at some widths. Truncation is by
    /// `char`, never by byte — the two implementations this replaces both
    /// sliced `&description[..60]`, which panics on any multi-byte
    /// codepoint at the boundary.
    pub subtitle_max_chars: Option<usize>,
}

impl Default for ListRowTokens {
    fn default() -> Self {
        Self {
            padding_x: 24.0,
            padding_y: 8.0,
            row_gap: 12.0,
            column_gap: 16.0,
            trail_min_width: 80.0,
            corner_radius: 2.0,
            subtitle_max_chars: Some(60),
        }
    }
}

/// Theme tokens a row and its paint system read.
#[derive(SystemParam)]
pub struct ListRowStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub spacing: Res<'w, Spacing>,
    pub tokens: Res<'w, ListRowTokens>,
}

/// What a row shows.
///
/// Builder-style, matching `TreeRowSpec`: enough of the parts are optional
/// that positional arguments stop reading.
#[derive(Clone, Debug, Default)]
pub struct ListRowSpec {
    id: String,
    title: String,
    meta: Option<String>,
    badge: Option<String>,
    subtitle: Option<String>,
}

impl ListRowSpec {
    /// A row identified by `id`, titled `title`.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            ..default()
        }
    }

    /// Muted text beside the title — a version, a count.
    #[must_use]
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// Accent-colored text beside the title — a kind, a format, a state.
    #[must_use]
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Second line under the title — a description, a category, a path.
    /// Truncated to [`ListRowTokens::subtitle_max_chars`].
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }
}

/// What [`spawn_list_row`] hands back.
///
/// Both entities are returned because both are seams: the caller attaches
/// its own observers to `row`, and parents its own widgets into `trail`.
#[derive(Clone, Copy, Debug)]
pub struct ListRowParts {
    /// The row itself. Observe this for clicks; it carries [`ListRowId`].
    pub row: Entity,
    /// The leading column, for extra content beside title/subtitle.
    pub lead: Entity,
    /// **The trailing slot — parent controls here.** Holds any number.
    pub trail: Entity,
}

/// Spawn a row. The caller parents it and attaches its own behaviour.
///
/// The row paints its own hover fill; everything else — click, drag,
/// context menu — is the caller's, attached to [`ListRowParts::row`].
pub fn spawn_list_row(
    commands: &mut Commands,
    style: &ListRowStyle,
    spec: ListRowSpec,
) -> ListRowParts {
    let tokens = &style.tokens;
    let palette = &style.palette;

    let row = commands
        .spawn((
            ListRow,
            ListRowId(spec.id),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(tokens.padding_x), Val::Px(tokens.padding_y)),
                margin: UiRect::bottom(Val::Px(tokens.row_gap)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(tokens.column_gap),
                border_radius: BorderRadius::all(Val::Px(tokens.corner_radius)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Interaction::default(),
            Name::new("tempera::list_row"),
        ))
        .id();

    let lead = commands
        .spawn((
            ListRowLead,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: Val::Px(2.0),
                overflow: Overflow::clip(),
                ..default()
            },
            ChildOf(row),
            bevy::picking::Pickable::IGNORE,
        ))
        .id();

    // Title line: title, then optional meta and badge beside it.
    let title_line = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(style.spacing.xs),
                ..default()
            },
            ChildOf(lead),
            bevy::picking::Pickable::IGNORE,
        ))
        .id();

    commands.spawn((
        ListRowTitle,
        Text::new(spec.title),
        style.font.text_font(style.typography.sm),
        TextColor(palette.foreground),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        ChildOf(title_line),
        bevy::picking::Pickable::IGNORE,
    ));

    if let Some(meta) = spec.meta {
        commands.spawn((
            ListRowMeta,
            Text::new(meta),
            style.font.text_font(style.typography.xs),
            TextColor(palette.muted_foreground),
            ChildOf(title_line),
            bevy::picking::Pickable::IGNORE,
        ));
    }

    if let Some(badge) = spec.badge {
        commands.spawn((
            ListRowBadge,
            Text::new(badge),
            style.font.text_font(style.typography.xs),
            TextColor(palette.primary),
            ChildOf(title_line),
            bevy::picking::Pickable::IGNORE,
        ));
    }

    if let Some(subtitle) = spec.subtitle {
        commands.spawn((
            ListRowSubtitle,
            Text::new(truncate(&subtitle, tokens.subtitle_max_chars)),
            style.font.text_font(style.typography.xs),
            TextColor(palette.muted_foreground),
            ChildOf(lead),
            bevy::picking::Pickable::IGNORE,
        ));
    }

    let trail = commands
        .spawn((
            ListRowTrail,
            Node {
                min_width: Val::Px(tokens.trail_min_width),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::End,
                column_gap: Val::Px(style.spacing.sm),
                ..default()
            },
            ChildOf(row),
        ))
        .id();

    ListRowParts { row, lead, trail }
}

/// Shorten `text` to `max` characters, ending in an ellipsis.
///
/// Counts `char`s rather than bytes so a multi-byte name is not cut
/// mid-codepoint. Text at exactly the cap is left alone — the ellipsis
/// only appears when something was actually removed.
pub(crate) fn truncate(text: &str, max: Option<usize>) -> String {
    let Some(max) = max.filter(|m| *m > 0) else {
        return text.to_string();
    };
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
