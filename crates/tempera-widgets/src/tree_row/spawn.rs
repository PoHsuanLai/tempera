use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::{
    ChevronState, TreeRow, TreeRowChevron, TreeRowExpanded, TreeRowHeader, TreeRowLabel,
    TreeRowSuffix,
};
use crate::theme::{Base, ColorPalette, FontHandle, Scale, Spacing, Step, Typography};

/// Row metrics, and the chevron art.
///
/// A resource rather than constants so an app can restyle every row at
/// once, and because the chevron images have to come from somewhere —
/// tempera ships no icon assets and should not start. A row spawned with
/// a chevron while these are `None` simply reserves the space, so layout
/// does not shift once the handles arrive.
#[derive(Resource, Clone, Debug)]
pub struct TreeRowTokens {
    /// Row height in logical pixels.
    pub height: f32,
    /// Left padding added per depth level.
    pub indent_step: f32,
    /// Icon and chevron edge length.
    pub icon_size: f32,
    /// Corner rounding on the hover fill.
    pub corner_radius: f32,
    /// Chevron pointing down.
    pub chevron_expanded: Option<Handle<Image>>,
    /// Chevron pointing right.
    pub chevron_collapsed: Option<Handle<Image>>,
    /// Hard character cap on the label, `None` to rely on flex alone.
    ///
    /// A cap exists because flex shrink plus text overflow races the
    /// suffix node and resolves to an overlap at some widths. Truncating
    /// by character count is cruder but predictable. The right value
    /// depends on the panel's width, which the widget does not know, so
    /// it is a token rather than a constant.
    pub label_max_chars: Option<usize>,
}

impl Default for TreeRowTokens {
    fn default() -> Self {
        Self::from_scale(Scale::new(Base::default()))
    }
}

impl TreeRowTokens {
    /// The defaults, generated from a spacing scale.
    ///
    /// `indent_step` and `corner_radius` are scale members — steps 2 and −2
    /// — so they are named rather than restated. The other two are not, and
    /// stay literals with the reason written down.
    ///
    /// Public fields on a `Resource`: overriding these is how a host tunes a
    /// tree for its own panel, and generating the default is not a reason to
    /// take that away.
    #[must_use]
    pub fn from_scale(scale: Scale) -> Self {
        let at = |n: i8| scale.at(Step::new(n)).get();
        Self {
            // Off the scale and off `Sizing` deliberately: a tree row is
            // denser than a control (`control_sm` is 28), and at 22 with a
            // 14px icon the gutter is 4 — step 0 — so the row is internally
            // coherent even though its own height is not a scale member.
            height: 22.0,
            indent_step: at(2),
            // Sized against the row rather than the grid, for the gutter
            // above.
            icon_size: 14.0,
            corner_radius: at(-2),
            chevron_expanded: None,
            chevron_collapsed: None,
            label_max_chars: Some(18),
        }
    }
}

/// Theme tokens a row and its paint systems read.
#[derive(SystemParam)]
pub struct TreeRowStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub spacing: Res<'w, Spacing>,
    pub tokens: Res<'w, TreeRowTokens>,
}

/// What a row shows.
///
/// Builder-style, mirroring `CommandItemSpec`: a row has enough optional
/// parts that positional arguments stop reading.
#[derive(Clone, Debug, Default)]
pub struct TreeRowSpec {
    label: String,
    suffix: Option<String>,
    icon: Option<Handle<Image>>,
    icon_tint: Option<Color>,
    depth: u16,
    chevron: Option<ChevronState>,
    header: bool,
}

impl TreeRowSpec {
    /// A plain row showing `label`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..default()
        }
    }

    /// Trailing text, rendered smaller and dimmer — an extension, a
    /// format, a count.
    #[must_use]
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Leading icon.
    #[must_use]
    pub fn icon(mut self, icon: Handle<Image>) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Multiply tint for the icon.
    ///
    /// Absent renders the image untouched, which is what a multi-color
    /// icon wants; a monochrome glyph passes a palette color.
    #[must_use]
    pub fn icon_tint(mut self, tint: Color) -> Self {
        self.icon_tint = Some(tint);
        self
    }

    /// Indent level. Multiplied by [`TreeRowTokens::indent_step`], so an
    /// app cannot disagree with itself about what one level is worth.
    #[must_use]
    pub fn depth(mut self, depth: u16) -> Self {
        self.depth = depth;
        self
    }

    /// Give this row a disclosure chevron, in `state`.
    #[must_use]
    pub fn chevron(mut self, state: ChevronState) -> Self {
        self.chevron = Some(state);
        self
    }

    /// Render at the brighter foreground token — for a row that names a
    /// group rather than an item inside one.
    #[must_use]
    pub fn header(mut self) -> Self {
        self.header = true;
        self
    }
}

/// Spawn a row. The caller parents it and attaches its own behaviour.
///
/// Returns the row entity. Children — chevron, icon, label, suffix — are
/// `Pickable::IGNORE` so a click resolves to the row itself rather than
/// to whichever part happened to be under the cursor.
pub fn spawn_tree_row(commands: &mut Commands, style: &TreeRowStyle, spec: TreeRowSpec) -> Entity {
    let tokens = &style.tokens;
    let palette = &style.palette;

    let row = commands
        .spawn((
            TreeRow,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(tokens.height),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(style.spacing.xs),
                padding: UiRect {
                    left: Val::Px(indent_px(tokens.indent_step, spec.depth)),
                    right: Val::Px(style.spacing.xs),
                    ..default()
                },
                border_radius: BorderRadius::all(Val::Px(tokens.corner_radius)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Interaction::default(),
            crate::cursor::HoverCursor::default(),
            Name::new("tempera::tree_row"),
        ))
        .id();

    if spec.header {
        commands.entity(row).insert(TreeRowHeader);
    }

    if let Some(state) = spec.chevron {
        if state.is_expanded() {
            commands.entity(row).insert(TreeRowExpanded);
        }
        let handle = chevron_handle(tokens, state);
        let mut chevron = commands.spawn((
            TreeRowChevron,
            Node {
                width: Val::Px(tokens.icon_size),
                height: Val::Px(tokens.icon_size),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(row),
            bevy::picking::Pickable::IGNORE,
        ));
        // Without art the node still occupies its slot, so the label does
        // not shift left and then jump right once the handles land.
        if let Some(handle) = handle {
            chevron.insert(ImageNode::new(handle).with_color(palette.muted_foreground));
        }
    }

    if let Some(icon) = spec.icon {
        commands.spawn((
            ImageNode::new(icon).with_color(spec.icon_tint.unwrap_or(palette.muted_foreground)),
            Node {
                width: Val::Px(tokens.icon_size),
                height: Val::Px(tokens.icon_size),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(row),
            bevy::picking::Pickable::IGNORE,
        ));
    }

    let label_color = if spec.header {
        palette.foreground
    } else {
        palette.muted_foreground
    };
    commands.spawn((
        TreeRowLabel,
        Text::new(truncate(&spec.label, tokens.label_max_chars)),
        style.font.text_font(style.typography.sm),
        TextColor(label_color),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        Node {
            flex_grow: 1.0,
            ..default()
        },
        ChildOf(row),
        bevy::picking::Pickable::IGNORE,
    ));

    if let Some(suffix) = spec.suffix {
        commands.spawn((
            TreeRowSuffix,
            Text::new(suffix),
            style.font.text_font(style.typography.xxs),
            TextColor(palette.muted_foreground),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(row),
            bevy::picking::Pickable::IGNORE,
        ));
    }

    row
}

/// Left padding for a row at `depth`.
///
/// Depth 0 still gets one step, so a flat list is inset from its
/// container edge rather than flush against it.
pub(crate) fn indent_px(step: f32, depth: u16) -> f32 {
    step * (depth as f32 + 1.0)
}

pub(crate) fn chevron_handle(tokens: &TreeRowTokens, state: ChevronState) -> Option<Handle<Image>> {
    match state {
        ChevronState::Expanded => tokens.chevron_expanded.clone(),
        ChevronState::Collapsed => tokens.chevron_collapsed.clone(),
    }
}

/// Shorten `label` to `max` characters, ending in an ellipsis.
///
/// Counts `char`s rather than bytes so a multi-byte name is not cut
/// mid-codepoint. A label at exactly the cap is left alone — the
/// ellipsis only appears when something was actually removed.
pub(crate) fn truncate(label: &str, max: Option<usize>) -> String {
    let Some(max) = max.filter(|m| *m > 0) else {
        return label.to_string();
    };
    if label.chars().count() <= max {
        return label.to_string();
    }
    let mut out: String = label.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_grows_one_step_per_level() {
        assert_eq!(indent_px(8.0, 0), 8.0);
        assert_eq!(indent_px(8.0, 1), 16.0);
        assert_eq!(indent_px(8.0, 3), 32.0);
    }

    #[test]
    fn a_short_label_is_left_alone() {
        assert_eq!(truncate("kick", Some(18)), "kick");
    }

    #[test]
    fn a_label_at_exactly_the_cap_is_not_truncated() {
        // The ellipsis means "something was removed"; at the cap nothing
        // was, so adding one would be a lie about the content.
        let label = "a".repeat(18);
        assert_eq!(truncate(&label, Some(18)), label);
    }

    #[test]
    fn a_long_label_ends_in_an_ellipsis() {
        let out = truncate(&"a".repeat(40), Some(18));
        assert_eq!(out.chars().count(), 18);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn truncation_counts_chars_not_bytes() {
        // Cutting on a byte index would split a multi-byte codepoint and
        // panic, or emit a replacement character.
        let out = truncate("日本語のファイル名です", Some(5));
        assert_eq!(out.chars().count(), 5);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn no_cap_means_no_truncation() {
        let long = "a".repeat(500);
        assert_eq!(truncate(&long, None), long);
        assert_eq!(truncate(&long, Some(0)), long, "a zero cap is not a cap");
    }
}
