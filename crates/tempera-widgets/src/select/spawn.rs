use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::{Select, SelectChevron, SelectDisplayText, SelectOption, SelectOptions, SelectValue};
use crate::theme::{ColorPalette, FontHandle, Typography};

#[derive(SystemParam)]
pub struct SelectStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub fonts: Res<'w, FontHandle>,
    pub typography: Res<'w, Typography>,
}

/// Spawn a select widget. Returns the root entity.
///
/// `options` is a list of `(id, label)` pairs. `selected` is the id of
/// the initially selected option.
pub fn spawn_select(
    commands: &mut Commands,
    style: &SelectStyle,
    options: Vec<SelectOption>,
    selected: &str,
) -> Entity {
    let display_label = options
        .iter()
        .find(|o| o.id == selected)
        .map(|o| o.label.clone())
        .unwrap_or_default();

    info!("[spawn_select] spawning Select with {} options, selected={}", options.len(), selected);
    let root = commands
        .spawn((
            Select,
            SelectValue(selected.to_string()),
            SelectOptions(options),
            Node {
                width: Val::Px(120.0),
                height: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
            BorderColor::all(style.palette.border),
            Interaction::default(),
            crate::cursor::HoverCursor::default(),
            Name::new("tempera::select"),
        ))
        .id();

    commands.spawn((
        SelectDisplayText,
        Text::new(display_label),
        style.fonts.text_font(style.typography.xs),
        TextColor(style.palette.foreground),
        bevy::picking::Pickable::IGNORE,
        ChildOf(root),
    ));

    commands.spawn((
        SelectChevron,
        Text::new("⌄"),
        style.fonts.text_font(style.typography.base),
        TextColor(style.palette.muted_foreground),
        bevy::picking::Pickable::IGNORE,
        Node {
            margin: UiRect::bottom(Val::Px(2.0)),
            ..default()
        },
        ChildOf(root),
    ));

    root
}
