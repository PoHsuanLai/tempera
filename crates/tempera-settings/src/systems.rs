//! Keeping the dialog's appearance in step with its state.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use tempera::theme::ColorPalette;

use crate::layout::SettingsLayout;
use crate::node::{SettingsBody, SettingsDialog, SettingsOpen, SettingsSidebar, SidebarEntry};
use crate::tab::{ActiveTab, TabBody, TabId};

/// Show the active tab's body and hide the rest.
///
/// **Mutates `node.display` in place.** The implementation this replaces
/// inserted a whole new `Node` per tab, which silently clobbered any layout
/// a tab body had set for itself — one tab's `Overflow::scroll_y()` was
/// dead after the first switch, and nothing pointed at the cause.
///
/// `Display::None`, not `Visibility::Hidden`: a hidden body must measure
/// zero, or the scroll extent of the visible one is computed against the
/// sum of all of them.
///
/// Watches `Changed<Children>` alongside `Changed<ActiveTab>` so a body
/// spawned *after* the dialog settled still gets hidden — `Added<Children>`
/// fires only on a container's first child ever.
pub(crate) fn apply_active_tab(
    dialogs: Query<
        (&ActiveTab, &Children),
        (
            With<SettingsDialog>,
            Or<(Changed<ActiveTab>, Changed<Children>)>,
        ),
    >,
    all_dialogs: Query<(&ActiveTab, &Children), With<SettingsDialog>>,
    added_bodies: Query<(), Added<TabBody>>,
    children: Query<&Children>,
    mut bodies: Query<(&TabId, &mut Node), With<TabBody>>,
) {
    let mut apply = |active: &ActiveTab, root: &Children| {
        // Bodies are grandchildren of the dialog root, not children, so the
        // walk is a small descent rather than a direct lookup.
        let mut stack: Vec<Entity> = root.iter().collect();
        while let Some(entity) = stack.pop() {
            if let Ok((id, mut node)) = bodies.get_mut(entity) {
                let want = if active.is(id.as_str()) {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.display != want {
                    node.display = want;
                }
                continue;
            }
            if let Ok(kids) = children.get(entity) {
                stack.extend(kids.iter());
            }
        }
    };

    for (active, root) in &dialogs {
        apply(active, root);
    }
    // A body added deeper in the tree does not mark the *root's* `Children`
    // as changed, so the query above can miss it entirely.
    if !added_bodies.is_empty() {
        for (active, root) in &all_dialogs {
            apply(active, root);
        }
    }
}

/// Paint the active sidebar entry and dim the rest.
pub(crate) fn repaint_sidebar(
    palette: Res<ColorPalette>,
    dialogs: Query<(&ActiveTab, &Children), With<SettingsDialog>>,
    changed: Query<(), (With<SettingsDialog>, Changed<ActiveTab>)>,
    added: Query<(), Added<SidebarEntry>>,
    children: Query<&Children>,
    mut entries: Query<(&SidebarEntry, &Interaction, &mut BackgroundColor)>,
    mut texts: Query<&mut TextColor>,
) {
    // The palette test is not optional bookkeeping. A theme swap makes every
    // entry stale at once, and that is not a fact about any entity — so it
    // cannot appear in `changed` or `added`, and without it the sidebar keeps
    // the old theme's accent until the user happens to click a different tab.
    if changed.is_empty() && added.is_empty() && !palette.is_changed() {
        return;
    }

    for (active, root) in &dialogs {
        let mut stack: Vec<Entity> = root.iter().collect();
        while let Some(entity) = stack.pop() {
            if let Ok((entry, interaction, mut bg)) = entries.get_mut(entity) {
                let selected = active.is(entry.0.as_str());
                let hovered = !matches!(interaction, Interaction::None);
                bg.0 = match (selected, hovered) {
                    (true, _) => palette.accent,
                    (false, true) => palette.muted,
                    (false, false) => Color::NONE,
                };
                let fg = if selected {
                    palette.accent_foreground
                } else {
                    palette.muted_foreground
                };
                if let Ok(kids) = children.get(entity) {
                    for kid in kids.iter() {
                        if let Ok(mut color) = texts.get_mut(kid) {
                            color.0 = fg;
                        }
                    }
                }
                continue;
            }
            if let Ok(kids) = children.get(entity) {
                stack.extend(kids.iter());
            }
        }
    }
}

/// Repaint a sidebar entry the pointer moved over.
///
/// Split from [`repaint_sidebar`] because hover changes far more often than
/// the active tab, and this one needs no tree walk.
pub(crate) fn repaint_sidebar_hover(
    palette: Res<ColorPalette>,
    mut entries: Query<
        (&SidebarEntry, &Interaction, &mut BackgroundColor, &ChildOf),
        Changed<Interaction>,
    >,
    parents: Query<&ChildOf>,
    dialogs: Query<&ActiveTab, With<SettingsDialog>>,
) {
    for (entry, interaction, mut bg, child_of) in &mut entries {
        // Find this entry's dialog so a hover cannot un-paint the selection.
        let mut cursor = child_of.parent();
        let active = loop {
            if let Ok(active) = dialogs.get(cursor) {
                break Some(active);
            }
            let Ok(parent) = parents.get(cursor) else {
                break None;
            };
            cursor = parent.parent();
        };
        let selected = active.is_some_and(|a| a.is(entry.0.as_str()));
        if selected {
            continue;
        }
        bg.0 = if matches!(interaction, Interaction::None) {
            Color::NONE
        } else {
            palette.muted
        };
    }
}

/// Mirror [`SettingsOpen`] onto the dialog root's `Visibility`.
///
/// This crate never *sets* `SettingsOpen` — see the crate docs. It only
/// reflects it, so a host that drives the flag from a menu, a keybind or a
/// command gets the same behaviour without this crate knowing which.
pub(crate) fn sync_visibility(
    mut dialogs: Query<
        (&SettingsOpen, &mut Visibility),
        (With<SettingsDialog>, Changed<SettingsOpen>),
    >,
) {
    for (open, mut visibility) in &mut dialogs {
        *visibility = if open.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Scroll the body of any open dialog under the wheel.
///
/// `MouseWheel` is `Option`al because a host may not have registered it —
/// a headless test, or an app built without `InputPlugin`. A hard
/// `MessageReader` panics the schedule in that case, which is a poor trade
/// for a feature that is only ever a convenience.
pub(crate) fn scroll_body(
    layout: Res<SettingsLayout>,
    wheel: Option<MessageReader<MouseWheel>>,
    dialogs: Query<(&SettingsOpen, &Children), With<SettingsDialog>>,
    children: Query<&Children>,
    mut bodies: Query<&mut ScrollPosition, With<SettingsBody>>,
) {
    let Some(mut wheel) = wheel else { return };
    let mut delta = 0.0;
    for ev in wheel.read() {
        delta += match ev.unit {
            MouseScrollUnit::Line => -ev.y * layout.scroll_speed,
            MouseScrollUnit::Pixel => -ev.y,
        };
    }
    if delta == 0.0 {
        return;
    }

    for (open, root) in &dialogs {
        if !open.0 {
            continue;
        }
        let mut stack: Vec<Entity> = root.iter().collect();
        while let Some(entity) = stack.pop() {
            if let Ok(mut scroll) = bodies.get_mut(entity) {
                scroll.0.y = (scroll.0.y + delta).max(0.0);
                continue;
            }
            if let Ok(kids) = children.get(entity) {
                stack.extend(kids.iter());
            }
        }
    }
}

/// Repaint the sidebar's own surface — its fill and its dividing rule.
///
/// Separate from [`repaint_sidebar`], which paints the *entries*. The entries
/// change on tab selection and hover; the surface changes only when the theme
/// does, so folding them together would run the hierarchy walk on every hover
/// to repaint two values that could not have moved.
///
/// tempera's dialog repaints its own backdrop and card. This is the one
/// surface that belongs to this crate rather than to the dialog, so it is the
/// one the dialog cannot reach.
pub(crate) fn repaint_sidebar_surface(
    palette: Res<ColorPalette>,
    mut sidebars: Query<(&mut BackgroundColor, &mut BorderColor), With<SettingsSidebar>>,
) {
    for (mut bg, mut border) in &mut sidebars {
        if bg.0 != palette.background {
            bg.0 = palette.background;
        }
        // `UiRect::right` at spawn, so only that edge is drawn — but the
        // colour is set on all four, and re-setting all four keeps this
        // agreeing with the spawn site rather than quietly differing from it.
        let want = BorderColor::all(palette.border);
        if *border != want {
            *border = want;
        }
    }
}
