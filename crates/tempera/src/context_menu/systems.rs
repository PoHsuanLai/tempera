//! Spawn the styled menu tree. Bevy's `MenuPlugin` handles keyboard
//! navigation, focus-loss dismissal, click activation, and modal
//! trapping; we only own visuals + the open-at-screen-position
//! convenience layer.

use bevy::diagnostic::FrameCount;
use bevy::input::ButtonInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, InteractionDisabled};
use bevy::ui_widgets::{Activate, MenuAction, MenuEvent, MenuItem as BevyMenuItem, MenuPopup};
use bevy::window::PrimaryWindow;

use super::components::{MenuRootMarker, TemperaMenuItem};
use super::request::{MenuItemSpec, MenuRequest};
use super::{MenuItemActivated, OpenContextMenu};
use crate::theme::MenuStyle;

const Z_MENU: i32 = 1000;
const WINDOW_PADDING: f32 = 4.0;

// ---------------------------------------------------------------------------
// Open at screen position
// ---------------------------------------------------------------------------

pub fn open_requested_menus(
    mut commands: Commands,
    mut events: MessageReader<OpenContextMenu>,
    existing: Query<Entity, With<MenuRootMarker>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    style: MenuStyle,
    frame: Res<FrameCount>,
) {
    let Some(OpenContextMenu(request)) = events.read().last().cloned() else {
        return;
    };

    for e in &existing {
        commands.entity(e).try_despawn();
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let anchor = place_anchor(
        request.anchor,
        window.width(),
        window.height(),
        &style,
        &request,
    );

    spawn_menu(&mut commands, anchor, &request, &style, frame.0);
}

/// Place the menu so it fits the window. Tries (anchor) first; flips
/// left of the anchor if it would clip the right edge, flips above if
/// it would clip the bottom. Falls back to clamping if the menu is
/// larger than the window.
fn place_anchor(anchor: Vec2, w: f32, h: f32, style: &MenuStyle, req: &MenuRequest) -> Vec2 {
    let menu_w = style.menu.width;
    let menu_h = estimate_height(req, style);

    let x = if anchor.x + menu_w + WINDOW_PADDING <= w {
        anchor.x
    } else if anchor.x - menu_w >= WINDOW_PADDING {
        anchor.x - menu_w
    } else {
        (w - menu_w - WINDOW_PADDING).max(WINDOW_PADDING)
    };

    let y = if anchor.y + menu_h + WINDOW_PADDING <= h {
        anchor.y
    } else if anchor.y - menu_h >= WINDOW_PADDING {
        anchor.y - menu_h
    } else {
        (h - menu_h - WINDOW_PADDING).max(WINDOW_PADDING)
    };

    Vec2::new(x, y)
}

fn estimate_height(req: &MenuRequest, style: &MenuStyle) -> f32 {
    let separators = req.items.iter().filter(|i| i.separator_before).count() as f32;
    let items = req.items.len() as f32;
    items * style.menu.item_height + separators * (1.0 + 8.0) + 8.0
}

fn spawn_menu(
    commands: &mut Commands,
    anchor: Vec2,
    request: &MenuRequest,
    style: &MenuStyle,
    frame: u32,
) {
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Px(anchor.x),
        top: Val::Px(anchor.y),
        width: Val::Px(style.menu.width),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(4.0)),
        border: UiRect::all(Val::Px(style.menu.border_width)),
        border_radius: BorderRadius::all(Val::Px(style.menu.corner_radius)),
        ..default()
    };

    // MenuPopup #[require]s TabGroup::modal(), which traps focus to
    // this subtree — clicking outside fires focus-loss and Bevy's
    // menu_on_lose_focus despawns the popup. We add the visual chrome
    // (background, border, rounded corners) and the z-order.
    let root = commands
        .spawn((
            MenuPopup::default(),
            MenuRootMarker {
                opened_at_frame: frame,
            },
            node,
            BackgroundColor(style.palette.popover),
            BorderColor::all(style.palette.border),
            GlobalZIndex(Z_MENU),
            FocusPolicy::Block,
            Name::new("tempera::context_menu"),
        ))
        .id();

    let mut tab_index = 0i32;
    for (idx, item) in request.items.iter().enumerate() {
        if item.separator_before && idx > 0 {
            spawn_separator(commands, root, style);
        }
        spawn_item(commands, root, item, tab_index, style);
        if item.enabled {
            tab_index += 1;
        }
    }
}

fn spawn_separator(commands: &mut Commands, parent: Entity, style: &MenuStyle) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(style.menu.separator),
        ChildOf(parent),
    ));
}

fn spawn_item(
    commands: &mut Commands,
    parent: Entity,
    spec: &MenuItemSpec,
    tab_index: i32,
    style: &MenuStyle,
) {
    let fg = if !spec.enabled {
        style.palette.muted_foreground
    } else if spec.destructive {
        style.palette.destructive
    } else {
        style.palette.popover_foreground
    };

    let row_node = Node {
        width: Val::Percent(100.0),
        height: Val::Px(style.menu.item_height),
        padding: UiRect::horizontal(Val::Px(style.menu.item_padding_x)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    };

    let mut row_cmds = commands.spawn((
        BevyMenuItem,
        TemperaMenuItem {
            id: spec.id.clone(),
        },
        Button,
        row_node,
        BackgroundColor(Color::NONE),
        ChildOf(parent),
    ));

    if spec.enabled {
        row_cmds.insert(TabIndex(tab_index));
    } else {
        row_cmds.insert(InteractionDisabled);
    }

    let row = row_cmds.id();

    commands.spawn((
        Text::new(spec.label.clone()),
        style.body_font(),
        TextColor(fg),
        Pickable::IGNORE,
        ChildOf(row),
    ));

    if let Some(chord) = &spec.shortcut {
        commands.spawn((
            Text::new(chord.clone()),
            style.shortcut_font(),
            TextColor(style.palette.muted_foreground),
            Pickable::IGNORE,
            ChildOf(row),
        ));
    }
}

// ---------------------------------------------------------------------------
// Hover & focus → highlight
// ---------------------------------------------------------------------------

/// Repaint row backgrounds based on hover + keyboard-focus state. One
/// system writes `BackgroundColor` so hover and arrow-key focus can't
/// fight over it.
pub fn paint_item_highlight(
    focus: Res<bevy::input_focus::InputFocus>,
    mut items: Query<
        (Entity, &Interaction, &mut BackgroundColor),
        (With<TemperaMenuItem>, With<BevyMenuItem>),
    >,
    menu: Res<crate::theme::MenuTokens>,
) {
    let focused = focus.0;
    for (entity, interaction, mut bg) in &mut items {
        let active = matches!(interaction, Interaction::Hovered | Interaction::Pressed)
            || focused == Some(entity);
        let color = if active {
            menu.item_hover_bg
        } else {
            Color::NONE
        };
        if bg.0 != color {
            *bg = BackgroundColor(color);
        }
    }
}

// ---------------------------------------------------------------------------
// Bevy's `Activate` → our `MenuItemActivated`
// ---------------------------------------------------------------------------

/// Bevy fires `Activate { entity }` when a `MenuItem` is clicked or
/// Enter/Space-activated. Translate that to our id-based message so
/// callers don't need to know about entity ids.
pub fn on_activate(
    trigger: On<Activate>,
    items: Query<&TemperaMenuItem>,
    mut writer: MessageWriter<MenuItemActivated>,
) {
    if let Ok(item) = items.get(trigger.entity) {
        writer.write(MenuItemActivated {
            id: item.id.clone(),
        });
    }
}

/// Bevy's `MenuPlugin` triggers `MenuAction::CloseAll` events when the
/// user presses Escape, clicks an item, etc., but assumes the menu's
/// *owner* (a `MenuButton` chain root) consumes them. Tempera context
/// menus are orphan popups with no such owner, so we close on the
/// `CloseAll` event ourselves.
///
/// `MenuEvent` is an `EntityEvent` with `auto_propagate` — it bubbles
/// up from the item that originated it through every ancestor. We
/// `try_despawn` (silent on missing entity) and stop propagation so
/// only the first root sighting closes the menu.
pub fn on_close_all(
    mut trigger: On<MenuEvent>,
    mut commands: Commands,
    roots: Query<(), With<MenuRootMarker>>,
) {
    if !matches!(trigger.action, MenuAction::CloseAll) {
        return;
    }
    if roots.get(trigger.source).is_ok() {
        commands.entity(trigger.source).try_despawn();
        trigger.propagate(false);
    }
}

// ---------------------------------------------------------------------------
// Same-frame open guard
// ---------------------------------------------------------------------------

/// Bevy's menu_on_lose_focus dismisses any `MenuPopup` whose focus
/// isn't on a descendant. On the frame the menu opens, focus hasn't
/// been assigned yet (menu_acquire_focus runs *next* Update), so the
/// menu would close immediately. This system grants the menu a
/// one-frame grace period by setting `InputFocus` to the menu entity
/// itself if focus is empty on the spawn frame.
pub fn seed_focus_on_open(
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    roots: Query<(Entity, &MenuRootMarker), Added<MenuRootMarker>>,
    frame: Res<FrameCount>,
) {
    for (entity, root) in &roots {
        if root.opened_at_frame == frame.0 && focus.0.is_none() {
            focus.0 = Some(entity);
        }
    }
}

// ---------------------------------------------------------------------------
// Dismiss on right-click outside (focus-loss covers left-click; right
// doesn't steal focus by default, so we handle it manually).
// ---------------------------------------------------------------------------

pub fn dismiss_on_outside_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    root: Query<(Entity, &MenuRootMarker, &ComputedNode, &GlobalTransform)>,
    frame: Res<FrameCount>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok((entity, menu, node, transform)) = root.single() else {
        return;
    };
    if frame.0.saturating_sub(menu.opened_at_frame) < 1 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let center = transform.translation().truncate();
    let half = node.size() * 0.5;
    let inside = (cursor.x - center.x).abs() <= half.x && (cursor.y - center.y).abs() <= half.y;

    if !inside {
        commands.entity(entity).try_despawn();
    }
}
