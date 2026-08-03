use bevy::ecs::system::SystemParam;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;

use super::components::{
    Command, CommandEmpty, CommandGroup, CommandGroupHeading, CommandInputRow, CommandItem,
    CommandItemSpec, CommandList, CommandSection, CommandSelection,
};
use crate::text_input::{TextInputStyle, spawn_text_input};
use crate::theme::{ColorPalette, ControlSize, FontHandle, Spacing, Step, Typography};

const PALETTE_WIDTH: f32 = 520.0;
const PALETTE_MAX_LIST_HEIGHT: f32 = 300.0;

/// Tokens read by the command-palette spawn + paint systems. Embeds
/// [`TextInputStyle`] so we can hand it through to `spawn_text_input`
/// for the search field rather than re-spawning a `TextInputNode` by
/// hand.
#[derive(SystemParam)]
pub struct CommandStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub spacing: Res<'w, Spacing>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub text_input: TextInputStyle<'w>,
}

/// Spawn a command palette as a child-of-window-style absolute Node.
/// Returns the root entity. Callers typically place the root inside a
/// modal backdrop and despawn it on Esc / activation.
///
/// Uses tempera's built-in node-drawn magnifier glyph for the search
/// icon. Use [`spawn_command_with_icon`] to substitute a rasterized
/// SVG or other `Handle<Image>` — handy for consumers (like dawai)
/// that already have an icon pipeline.
///
/// The tree mirrors shadcn's `<Command>` shape:
///
/// ```text
/// Command (root)
/// ├── CommandInputRow
/// │   ├── search icon (optional, not spawned here — caller can add)
/// │   └── TextInputNode
/// ├── CommandList
/// │   ├── CommandEmpty (hidden by default)
/// │   ├── CommandGroup
/// │   │   ├── CommandGroupHeading (text)
/// │   │   └── CommandItem (...)
/// │   └── ...
/// ```
pub fn spawn_command(
    commands: &mut Commands,
    style: &CommandStyle,
    placeholder: impl Into<String>,
    sections: Vec<CommandSection>,
) -> Entity {
    spawn_command_with_icon(commands, style, placeholder, None, sections)
}

/// Variant of [`spawn_command`] that lets the caller supply the
/// search icon as a `Handle<Image>`. Pass `None` to fall back to
/// tempera's built-in node-drawn glyph. Dawai uses this to plug in
/// its rasterized `search.svg` from the existing icon pipeline.
pub fn spawn_command_with_icon(
    commands: &mut Commands,
    style: &CommandStyle,
    placeholder: impl Into<String>,
    icon: Option<Handle<Image>>,
    sections: Vec<CommandSection>,
) -> Entity {
    let label_font = style.font.text_font(style.typography.sm);
    let heading_font = style.font.text_font(style.typography.xs);
    let item_font = style.font.text_font(style.typography.sm);

    let root = commands
        .spawn((
            Command,
            CommandSelection::default(),
            Node {
                width: Val::Px(PALETTE_WIDTH),
                flex_direction: FlexDirection::Column,
                border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small)),
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::clip(),
                // Intrinsic height — shrink to content. Without this,
                // a column-flex parent's default cross-axis stretch
                // would expand the palette to fill the viewport.
                align_self: AlignSelf::FlexStart,
                ..default()
            },
            BackgroundColor(style.palette.popover),
            BorderColor::all(style.palette.border),
            // Stop click-to-focus from falling through to the window
            // and clearing focus, matching the tooltip / dropdown
            // story.
            TabIndex(0),
            Name::new("tempera::command"),
        ))
        .id();

    spawn_input_row(commands, root, style, &label_font, placeholder.into(), icon);

    let list = commands
        .spawn((
            CommandList,
            Node {
                flex_direction: FlexDirection::Column,
                max_height: Val::Px(PALETTE_MAX_LIST_HEIGHT),
                overflow: Overflow::scroll_y(),
                padding: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(root),
            Name::new("tempera::command::list"),
        ))
        .id();

    spawn_empty_placeholder(commands, list, style, &label_font);

    for section in sections {
        spawn_section(commands, list, style, &heading_font, &item_font, section);
    }

    root
}

fn spawn_input_row(
    commands: &mut Commands,
    root: Entity,
    style: &CommandStyle,
    _label_font: &TextFont,
    placeholder: String,
    icon: Option<Handle<Image>>,
) {
    let row = commands
        .spawn((
            CommandInputRow,
            Node {
                height: style.text_input.metrics.control(ControlSize::Lg).into(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(style.spacing.xs),
                padding: UiRect::horizontal(Val::Px(12.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(style.palette.border),
            ChildOf(root),
            Name::new("tempera::command::input_row"),
        ))
        .id();

    if let Some(handle) = icon {
        // Caller-supplied icon. Tint with `muted_foreground` if the
        // asset is white/alpha; if it's already colored the multiply
        // still composes sensibly.
        commands.spawn((
            ImageNode::new(handle).with_color(style.palette.muted_foreground),
            Node {
                width: Val::Px(16.0),
                height: Val::Px(16.0),
                ..default()
            },
            ChildOf(row),
        ));
    } else {
        spawn_search_glyph(commands, row, style.palette.muted_foreground);
    }

    // Use tempera's `spawn_text_input` so we get the same I-beam
    // cursor, focus-on-click, and TextInputContents wiring as the
    // standalone text input. Strip the surround's chrome (border /
    // bg / padding) so the input blends into the row.
    let handle = spawn_text_input(commands, &style.text_input, "", placeholder);
    commands.entity(handle.surround).insert((
        Node {
            flex_grow: 1.0,
            height: Val::Auto,
            border: UiRect::ZERO,
            padding: UiRect::ZERO,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        ChildOf(row),
    ));
}

fn spawn_empty_placeholder(
    commands: &mut Commands,
    list: Entity,
    style: &CommandStyle,
    label_font: &TextFont,
) {
    let row = commands
        .spawn((
            CommandEmpty,
            Node {
                // Hidden by default — filter system flips this on when
                // no items match.
                display: Display::None,
                padding: UiRect::vertical(Val::Px(24.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(list),
            Name::new("tempera::command::empty"),
        ))
        .id();

    commands.spawn((
        Text::new("No results found.".to_string()),
        label_font.clone(),
        TextColor(style.palette.muted_foreground),
        bevy::picking::Pickable::IGNORE,
        ChildOf(row),
    ));
}

fn spawn_section(
    commands: &mut Commands,
    list: Entity,
    style: &CommandStyle,
    heading_font: &TextFont,
    item_font: &TextFont,
    section: CommandSection,
) {
    let group = commands
        .spawn((
            CommandGroup,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(
                    style.text_input.metrics.gap(Step::new(2)).into(),
                    style.text_input.metrics.gap(Step::BASE).into(),
                ),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(list),
            Name::new("tempera::command::group"),
        ))
        .id();

    // Heading row.
    let heading = commands
        .spawn((
            CommandGroupHeading,
            Node {
                padding: UiRect::axes(Val::Px(4.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(group),
        ))
        .id();
    commands.spawn((
        Text::new(section.heading.clone()),
        heading_font.clone(),
        TextColor(style.palette.muted_foreground),
        bevy::picking::Pickable::IGNORE,
        ChildOf(heading),
    ));

    for spec in section.items {
        spawn_item(commands, group, style, item_font, spec);
    }
}

fn spawn_item(
    commands: &mut Commands,
    group: Entity,
    style: &CommandStyle,
    item_font: &TextFont,
    spec: CommandItemSpec,
) {
    let item = commands
        .spawn((
            CommandItem {
                search_text: spec.label.to_lowercase(),
                id: spec.id.clone(),
                disabled: spec.disabled,
            },
            Node {
                height: style.text_input.metrics.control(ControlSize::Md).into(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                column_gap: Val::Px(style.spacing.xs),
                border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Interaction::default(),
            crate::cursor::HoverCursor::default(),
            ChildOf(group),
            Name::new("tempera::command::item"),
        ))
        .id();

    commands.spawn((
        Text::new(spec.label),
        item_font.clone(),
        TextColor(if spec.disabled {
            with_alpha(style.palette.foreground, 0.5)
        } else {
            style.palette.foreground
        }),
        Node {
            flex_grow: 1.0,
            ..default()
        },
        bevy::picking::Pickable::IGNORE,
        ChildOf(item),
    ));

    // Shortcut chip — inline kbd-like rendering. Right-aligned via
    // `flex_grow: 1` on the label above pushing this to the end.
    if let Some(chord) = spec.shortcut {
        crate::kbd::spawn_chord_inline(
            commands,
            item,
            &chord,
            &style.font,
            &style.typography,
            style.palette.muted_foreground,
        );
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}

/// Tiny magnifier-ish glyph built out of plain UI nodes — a hollow
/// ring + a small dot at its bottom-right approximating the handle's
/// stub. Bevy UI doesn't rotate flat-colored Nodes (no rotation in
/// `UiTransform`), so a true magnifier needs a rasterized SVG —
/// callers can supply one via [`super::spawn_command_with_icon`].
fn spawn_search_glyph(commands: &mut Commands, parent: Entity, color: Color) {
    const GLYPH_BOX: f32 = 16.0;
    const RING_DIAMETER: f32 = 10.0;
    const RING_STROKE: f32 = 1.5;
    const DOT_SIZE: f32 = 3.0;

    let wrap = commands
        .spawn((
            Node {
                width: Val::Px(GLYPH_BOX),
                height: Val::Px(GLYPH_BOX),
                position_type: PositionType::Relative,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(parent),
        ))
        .id();

    // Lens — hollow ring.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(1.5),
            left: Val::Px(1.5),
            width: Val::Px(RING_DIAMETER),
            height: Val::Px(RING_DIAMETER),
            border: UiRect::all(Val::Px(RING_STROKE)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(color),
        ChildOf(wrap),
    ));

    // Handle stub — a small filled dot at the ring's bottom-right.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(GLYPH_BOX - DOT_SIZE - 1.5),
            left: Val::Px(GLYPH_BOX - DOT_SIZE - 1.5),
            width: Val::Px(DOT_SIZE),
            height: Val::Px(DOT_SIZE),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(color),
        ChildOf(wrap),
    ));
}
