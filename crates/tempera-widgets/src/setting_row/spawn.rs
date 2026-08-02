use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::{
    SettingRow, SettingRowControl, SettingRowDescription, SettingRowLabel, SettingSection,
};
use crate::theme::{ColorPalette, FontHandle, Spacing, Typography};

/// Row metrics.
///
/// A resource rather than constants so an app restyles every form at once,
/// and so `control_width` is written in exactly one place. In the code this
/// widget replaces, a `CONTROL_WIDTH` constant existed *and* the literal
/// `Val::Px(200.0)` was hand-written at eight call sites to force spawned
/// selects to match it — the two had already drifted apart in spelling, and
/// nothing would have caught them drifting apart in value.
#[derive(Resource, Clone, Debug)]
pub struct SettingRowTokens {
    /// Horizontal padding inside a row and a section header.
    pub padding_x: f32,
    /// Vertical padding inside a row.
    pub padding_y: f32,
    /// Gap below each row.
    pub row_gap: f32,
    /// Gap above each section header.
    pub section_gap: f32,
    /// Minimum height of a row.
    pub row_height: f32,
    /// Width of the control slot. Fixed so controls align down the form.
    pub control_width: f32,
}

impl Default for SettingRowTokens {
    fn default() -> Self {
        Self {
            padding_x: 24.0,
            padding_y: 8.0,
            row_gap: 12.0,
            section_gap: 18.0,
            row_height: 36.0,
            control_width: 200.0,
        }
    }
}

/// Theme tokens a settings row reads.
#[derive(SystemParam)]
pub struct SettingRowStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub spacing: Res<'w, Spacing>,
    pub tokens: Res<'w, SettingRowTokens>,
}

/// What a row shows.
#[derive(Clone, Debug, Default)]
pub struct SettingRowSpec {
    label: String,
    description: Option<String>,
}

impl SettingRowSpec {
    /// A row labelled `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..default()
        }
    }

    /// A second, dimmer line explaining what the setting does.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Spawn a settings row, returning **the control slot**.
///
/// Only the slot is returned because only the slot is a seam. The row
/// entity was returned by the implementation this replaces and discarded at
/// every one of its 21 call sites, so returning it here would be a tuple
/// nobody destructures.
///
/// The row parents itself into `parent`, so the caller never holds it.
///
/// ```ignore
/// let slot = spawn_setting_row(
///     &mut commands,
///     &style,
///     body,
///     SettingRowSpec::new("Auto Save").description("Save every 5 minutes"),
/// );
/// let sw = spawn_switch(&mut commands, &switch_style, enabled);
/// commands.entity(sw).insert(ChildOf(slot)).observe(/* … */);
/// ```
pub fn spawn_setting_row(
    commands: &mut Commands,
    style: &SettingRowStyle,
    parent: Entity,
    spec: SettingRowSpec,
) -> Entity {
    let tokens = &style.tokens;
    let palette = &style.palette;

    let row = commands
        .spawn((
            SettingRow,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(tokens.padding_x), Val::Px(tokens.padding_y)),
                margin: UiRect::bottom(Val::Px(tokens.row_gap)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(style.spacing.md),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    let label_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: Val::Px(2.0),
                ..default()
            },
            ChildOf(row),
        ))
        .id();

    commands.spawn((
        SettingRowLabel,
        Text::new(spec.label),
        style.font.text_font(style.typography.sm),
        TextColor(palette.foreground),
        ChildOf(label_col),
    ));

    if let Some(description) = spec.description {
        commands.spawn((
            SettingRowDescription,
            Text::new(description),
            style.font.text_font(style.typography.xs),
            TextColor(palette.muted_foreground),
            ChildOf(label_col),
        ));
    }

    commands
        .spawn((
            SettingRowControl,
            Node {
                width: Val::Px(tokens.control_width),
                min_height: Val::Px(tokens.row_height - 8.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::End,
                ..default()
            },
            ChildOf(row),
        ))
        .id()
}

/// Spawn an all-caps section heading above a group of rows.
///
/// Returns the heading entity for symmetry with the rest of the crate; the
/// implementation this replaces returned it and was never once asked for it.
pub fn spawn_section_header(
    commands: &mut Commands,
    style: &SettingRowStyle,
    parent: Entity,
    text: &str,
) -> Entity {
    let tokens = &style.tokens;
    let header = commands
        .spawn((
            SettingSection,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(tokens.padding_x), Val::Px(8.0)),
                margin: UiRect::top(Val::Px(tokens.section_gap)),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        Text::new(text.to_string()),
        style.font.text_font(style.typography.xs),
        TextColor(style.palette.muted_foreground),
        ChildOf(header),
    ));

    header
}
