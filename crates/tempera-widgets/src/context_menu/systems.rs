//! Spawn the styled menu tree. Bevy's `MenuPlugin` handles keyboard
//! navigation, focus-loss dismissal, click activation, and modal
//! trapping; we only own visuals + the open-at-screen-position
//! convenience layer.

use bevy::diagnostic::FrameCount;
use bevy::input::ButtonInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::ui::{FocusPolicy, InteractionDisabled};
use bevy::ui_widgets::{Activate, MenuAction, MenuEvent, MenuItem as BevyMenuItem, MenuPopup};
use bevy::window::PrimaryWindow;

use super::components::{HasSubMenu, MenuRootMarker, SubMenuChild, SubMenuOf, TemperaMenuItem};
use super::request::{MenuItemSpec, MenuRequest};
use super::{MenuItemActivated, OpenContextMenu};
use crate::menu_tokens::MenuStyle;

const Z_MENU: i32 = 3000;
/// Gap kept between a menu and the window edge when it has to be nudged
/// back on-screen.
///
/// Step 0 on the spacing scale — 4 at the default base. Kept as a const
/// because `clamp_to_window` below is a pure function used by both the open
/// path and the submenu path, and threading a resource through it to save one
/// literal would cost more clarity than it buys. Revisit if the flip logic
/// ever grows a second spacing value.
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
) -> Entity {
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

    let has_children = spec.has_children();

    let mut row_cmds = commands.spawn((
        TemperaMenuItem {
            id: spec.id.clone(),
            origin: spec.origin,
        },
        Button,
        row_node,
        BackgroundColor(Color::NONE),
        ChildOf(parent),
    ));

    if has_children {
        row_cmds.insert(HasSubMenu(spec.children.clone()));
    } else {
        // Only leaf items get BevyMenuItem (which fires Activate on
        // click and triggers CloseAll). Submenu parents must not
        // dismiss the menu on click.
        row_cmds.insert(BevyMenuItem);
        if spec.enabled {
            row_cmds.insert(TabIndex(tab_index));
        }
    }

    if !spec.enabled {
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

    if has_children {
        commands.spawn((
            Text::new("›"),
            style.body_font(),
            TextColor(style.palette.muted_foreground),
            Pickable::IGNORE,
            ChildOf(row),
        ));
    } else if let Some(chord) = &spec.shortcut {
        crate::kbd::spawn_chord_inline(
            commands,
            row,
            chord,
            &style.font,
            &style.typography,
            style.palette.muted_foreground,
        );
    }

    row
}

// ---------------------------------------------------------------------------
// Submenu open / close on hover
// ---------------------------------------------------------------------------

/// Marker inserted on a HasSubMenu item when the pointer enters it.
/// Cleared when the close timer fires. Drives submenu open/close.
#[derive(Component)]
pub(crate) struct SubMenuHovered;

/// Delay before closing a submenu after the pointer leaves the parent
/// item. Gives the user time to move the cursor into the submenu.
#[derive(Component)]
pub(crate) struct SubMenuCloseTimer {
    pub deadline: f64,
}

const SUBMENU_CLOSE_DELAY: f64 = 0.15;

pub fn observe_submenu_hover(app: &mut bevy::app::App) {
    use bevy::picking::events::{Out, Over, Pointer};

    // Pointer enters a HasSubMenu item → mark hovered, cancel any pending close.
    app.add_observer(
        |trigger: On<Pointer<Over>>, mut commands: Commands, q: Query<(), With<HasSubMenu>>| {
            if q.contains(trigger.entity) {
                commands
                    .entity(trigger.entity)
                    .insert(SubMenuHovered)
                    .remove::<SubMenuCloseTimer>();
            }
        },
    );

    // Pointer leaves a HasSubMenu item → start close timer instead of
    // removing SubMenuHovered immediately.
    app.add_observer(
        |trigger: On<Pointer<Out>>,
         mut commands: Commands,
         q: Query<(), With<HasSubMenu>>,
         time: Res<Time>| {
            if q.contains(trigger.entity) {
                commands.entity(trigger.entity).insert(SubMenuCloseTimer {
                    deadline: time.elapsed_secs_f64() + SUBMENU_CLOSE_DELAY,
                });
            }
        },
    );

    // Pointer enters a submenu child → cancel the parent's close timer.
    app.add_observer(
        |trigger: On<Pointer<Over>>,
         mut commands: Commands,
         children: Query<(), With<SubMenuChild>>,
         subs: Query<&SubMenuOf>,
         timers: Query<(), With<SubMenuCloseTimer>>| {
            if !children.contains(trigger.entity) {
                return;
            }
            // Find which parent item owns this submenu and cancel its timer.
            for sub in subs.iter() {
                if timers.contains(sub.0) {
                    commands.entity(sub.0).remove::<SubMenuCloseTimer>();
                }
            }
        },
    );

    // Pointer enters a regular menu item (not a submenu parent, not
    // inside a submenu) → close all submenus immediately.
    app.add_observer(
        |trigger: On<Pointer<Over>>,
         mut commands: Commands,
         items: Query<
            (),
            (
                With<TemperaMenuItem>,
                Without<HasSubMenu>,
                Without<SubMenuChild>,
            ),
        >,
         subs: Query<Entity, With<SubMenuOf>>,
         timers: Query<Entity, With<SubMenuCloseTimer>>| {
            if items.contains(trigger.entity) {
                for sub in subs.iter() {
                    commands.entity(sub).try_despawn();
                }
                for timer_ent in timers.iter() {
                    commands.entity(timer_ent).remove::<SubMenuCloseTimer>();
                    commands.entity(timer_ent).remove::<SubMenuHovered>();
                }
            }
        },
    );
}

/// Tick close timers: when a timer fires, remove SubMenuHovered and
/// despawn the associated submenu.
pub fn tick_submenu_close_timers(
    mut commands: Commands,
    timers: Query<(Entity, &SubMenuCloseTimer)>,
    subs: Query<(Entity, &SubMenuOf)>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    for (item_entity, timer) in timers.iter() {
        if now >= timer.deadline {
            commands
                .entity(item_entity)
                .remove::<SubMenuCloseTimer>()
                .remove::<SubMenuHovered>();
            for (sub_entity, parent) in subs.iter() {
                if parent.0 == item_entity {
                    commands.entity(sub_entity).try_despawn();
                }
            }
        }
    }
}

pub fn manage_submenus(
    mut commands: Commands,
    hovered_items: Query<
        (Entity, &HasSubMenu, &ComputedNode, &UiGlobalTransform),
        With<SubMenuHovered>,
    >,
    existing_subs: Query<(Entity, &SubMenuOf)>,
    menu_root: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<MenuRootMarker>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    style: MenuStyle,
) {
    let scale = windows.single().map(|w| w.scale_factor()).unwrap_or(1.0);

    for (item_entity, sub, node, ui_gt) in hovered_items.iter() {
        let already_open = existing_subs
            .iter()
            .any(|(_, parent)| parent.0 == item_entity);
        if already_open {
            continue;
        }

        let Ok((root_entity, root_node, root_gt)) = menu_root.single() else {
            continue;
        };

        // Close any other open submenus.
        for (sub_entity, _) in existing_subs.iter() {
            commands.entity(sub_entity).try_despawn();
        }

        // All transforms are in physical pixels; convert to logical.
        // UiGlobalTransform.translation is the node's center.
        // Absolute children position from the parent's top-left corner.
        let root_center = root_gt.translation / scale;
        let root_half = root_node.size() / scale * 0.5;
        let root_top_left = root_center - root_half;

        let item_center = ui_gt.translation / scale;
        let item_half = node.size() / scale * 0.5;

        let anchor = Vec2::new(
            item_center.x + item_half.x - root_top_left.x,
            item_center.y - item_half.y - root_top_left.y,
        );
        let sub_root = commands
            .spawn((
                SubMenuOf(item_entity),
                SubMenuChild,
                ChildOf(root_entity),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(anchor.x),
                    top: Val::Px(anchor.y),
                    width: Val::Px(style.menu.width),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(4.0)),
                    border: UiRect::all(Val::Px(style.menu.border_width)),
                    border_radius: BorderRadius::all(Val::Px(style.menu.corner_radius)),
                    ..default()
                },
                BackgroundColor(style.palette.popover),
                BorderColor::all(style.palette.border),
                GlobalZIndex(Z_MENU + 1),
                FocusPolicy::Block,
                Name::new("tempera::context_submenu"),
            ))
            .id();

        let mut tab_index = 0i32;
        for (idx, child_spec) in sub.0.iter().enumerate() {
            if child_spec.separator_before && idx > 0 {
                spawn_separator(&mut commands, sub_root, &style);
            }
            let item = spawn_item(&mut commands, sub_root, child_spec, tab_index, &style);
            commands.entity(item).insert(SubMenuChild);
            if child_spec.enabled {
                tab_index += 1;
            }
        }
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
    mut items: Query<(Entity, &Interaction, &mut BackgroundColor), With<TemperaMenuItem>>,
    menu: Res<crate::menu_tokens::MenuTokens>,
) {
    let focused = focus.get();
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
            entity: item.origin,
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
        if root.opened_at_frame == frame.0 {
            focus.set(entity, bevy::input_focus::FocusCause::Navigated);
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
