use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui_widgets::{Checkbox, RadioButton, RadioGroup};

use super::components::{ToggleGroup, ToggleGroupKind, ToggleItem};
use crate::theme::{ColorPalette, ControlSize, FontHandle, Metrics, Spacing, Step, Typography};

/// Tokens read by toggle-group spawn / paint systems.
#[derive(SystemParam)]
pub struct ToggleGroupStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub metrics: Metrics<'w>,
    pub spacing: Res<'w, Spacing>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
}

/// Caller-supplied item config. `selected` is the *initial* state;
/// flips after spawn happen via Bevy's behavior layer + tempera's
/// `radio_group_self_update` observer.
#[derive(Clone, Debug)]
pub struct ToggleGroupItem {
    pub label: String,
    pub selected: bool,
}

impl ToggleGroupItem {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
        }
    }

    #[must_use]
    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }
}

/// Spawn a toggle group. Returns the root entity (carries
/// [`ToggleGroup`] + [`ToggleGroupKind`] + the appropriate Bevy
/// behavior component).
///
/// Item entities are children of the root. For Single mode, observe
/// `ValueChange<Entity>` on the root to react to selection changes.
/// For Multiple mode, observe `ValueChange<bool>` on each item.
pub fn spawn_toggle_group(
    commands: &mut Commands,
    style: &ToggleGroupStyle,
    kind: ToggleGroupKind,
    items: Vec<ToggleGroupItem>,
) -> Entity {
    let mut root = commands.spawn((
        ToggleGroup,
        kind,
        Node {
            flex_direction: FlexDirection::Row,
            border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small)),
            ..default()
        },
        // shadcn toggle groups have no track background — items render
        // their own surface (transparent / muted / accent). Setting a
        // bg here would collapse contrast between hovered and selected
        // items in dark mode.
        BackgroundColor(Color::NONE),
        Name::new("tempera::toggle_group"),
    ));

    match kind {
        ToggleGroupKind::Single => {
            root.insert(RadioGroup);
        }
        ToggleGroupKind::Multiple => {}
    }
    let root_id = root.id();

    for item in items {
        spawn_item(commands, root_id, &item, kind, style);
    }

    root_id
}

fn spawn_item(
    commands: &mut Commands,
    parent: Entity,
    item: &ToggleGroupItem,
    kind: ToggleGroupKind,
    style: &ToggleGroupStyle,
) {
    let mut e = commands.spawn((
        ToggleItem,
        Node {
            height: style.metrics.control(ControlSize::Sm).into(),
            padding: UiRect::horizontal(Val::Px(style.metrics.gap(Step::new(3)).get())),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small - 2.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Interaction::default(),
        crate::cursor::HoverCursor::default(),
        ChildOf(parent),
    ));

    match kind {
        ToggleGroupKind::Single => {
            e.insert(RadioButton);
        }
        ToggleGroupKind::Multiple => {
            e.insert(Checkbox);
        }
    }
    if item.selected {
        e.insert(bevy::ui::Checked);
    }
    let item_id = e.id();

    commands.spawn((
        Text::new(item.label.clone()),
        style.font.text_font(style.typography.sm),
        TextColor(style.palette.foreground),
        bevy::picking::Pickable::IGNORE,
        ChildOf(item_id),
    ));
}
