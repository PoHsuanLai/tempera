use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::FocusedInput;
use bevy::input_focus::InputFocus;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy_ui_text_input::TextInputContents;

use super::components::{
    Command, CommandActivated, CommandEmpty, CommandGroup, CommandGroupHeading, CommandInputRow,
    CommandItem, CommandList, CommandSelection,
};
use crate::theme::ColorPalette;

/// Click on an item activates it. Routes to the command-root via the
/// item's ancestry (item → group → list → command root).
pub(crate) fn on_item_click(
    on: On<Pointer<Click>>,
    items: Query<(&CommandItem, &ChildOf), Without<CommandGroupHeading>>,
    groups: Query<&ChildOf, With<CommandGroup>>,
    lists: Query<&ChildOf, With<CommandList>>,
    palettes: Query<Entity, With<Command>>,
    mut commands: Commands,
) {
    let Ok((item, item_parent)) = items.get(on.entity) else {
        return;
    };
    if item.disabled {
        return;
    }
    let Ok(group_parent) = groups.get(item_parent.0) else {
        return;
    };
    let Ok(list_parent) = lists.get(group_parent.0) else {
        return;
    };
    let Ok(palette) = palettes.get(list_parent.0) else {
        return;
    };
    commands.trigger(CommandActivated {
        palette,
        id: item.id.clone(),
    });
}

/// Move selection on Up/Down, activate on Enter. Listens for focused
/// keyboard input on the palette root.
pub(crate) fn on_keyboard(
    on: On<FocusedInput<KeyboardInput>>,
    palettes: Query<&Children, With<Command>>,
    lists: Query<&Children, With<CommandList>>,
    groups: Query<&Children, With<CommandGroup>>,
    items: Query<(Entity, &CommandItem, &Node)>,
    mut sels: Query<&mut CommandSelection>,
    mut commands: Commands,
) {
    let input = &on.event().input;
    if !input.state.is_pressed() {
        return;
    }
    let Ok(palette_kids) = palettes.get(on.focused_entity) else {
        return;
    };

    let visible_items = collect_visible_items(palette_kids, &lists, &groups, &items);
    if visible_items.is_empty() {
        return;
    }

    let Ok(mut sel) = sels.get_mut(on.focused_entity) else {
        return;
    };

    match input.key_code {
        KeyCode::ArrowDown => {
            let next = step_selection(&visible_items, sel.selected, 1);
            sel.selected = Some(next);
        }
        KeyCode::ArrowUp => {
            let next = step_selection(&visible_items, sel.selected, -1);
            sel.selected = Some(next);
        }
        KeyCode::Enter => {
            let target = sel.selected.or_else(|| visible_items.first().copied());
            if let Some(target) = target
                && let Ok((_, item, _)) = items.get(target)
                && !item.disabled
            {
                commands.trigger(CommandActivated {
                    palette: on.focused_entity,
                    id: item.id.clone(),
                });
            }
        }
        _ => {}
    }
}

fn collect_visible_items(
    palette_kids: &Children,
    lists: &Query<&Children, With<CommandList>>,
    groups: &Query<&Children, With<CommandGroup>>,
    items: &Query<(Entity, &CommandItem, &Node)>,
) -> Vec<Entity> {
    let mut out = Vec::new();
    for list_child in palette_kids.iter() {
        let Ok(list_kids) = lists.get(list_child) else {
            continue;
        };
        for group_child in list_kids.iter() {
            let Ok(group_kids) = groups.get(group_child) else {
                continue;
            };
            // Skip hidden groups.
            for item_e in group_kids.iter() {
                let Ok((entity, item, node)) = items.get(item_e) else {
                    continue;
                };
                if node.display == Display::None || item.disabled {
                    continue;
                }
                out.push(entity);
            }
        }
    }
    out
}

fn step_selection(items: &[Entity], current: Option<Entity>, delta: i32) -> Entity {
    let cur_idx = current
        .and_then(|e| items.iter().position(|x| *x == e))
        .map(|i| i as i32);
    let n = items.len() as i32;
    let next = match cur_idx {
        Some(i) => (i + delta).rem_euclid(n),
        None if delta > 0 => 0,
        None => n - 1,
    };
    items[next as usize]
}

/// Refilter visible items each time the input's text changes. Empty
/// query = show everything. Otherwise show items whose lowercased
/// label contains the lowercased query. Groups with no visible items
/// are hidden too. Empty-state placeholder is shown when nothing
/// matches.
pub(crate) fn refilter(
    palettes: Query<(Entity, &Children), With<Command>>,
    input_rows: Query<&Children, With<CommandInputRow>>,
    inputs: Query<&TextInputContents, Changed<TextInputContents>>,
    all_inputs: Query<&TextInputContents>,
    children_q: Query<&Children>,
    lists: Query<&Children, With<CommandList>>,
    mut groups: Query<(&mut Node, &Children), (With<CommandGroup>, Without<CommandItem>, Without<CommandEmpty>)>,
    mut items: Query<(&CommandItem, &mut Node), Without<CommandGroup>>,
    mut empty: Query<&mut Node, (With<CommandEmpty>, Without<CommandGroup>, Without<CommandItem>)>,
    mut sels: Query<&mut CommandSelection>,
) {
    // Determine which palettes need a refilter. A palette refilters
    // when its input's `TextInputContents` changed this frame.
    for (palette, palette_kids) in &palettes {
        let input_entity = find_input_entity(palette_kids, &input_rows, &all_inputs, &children_q);
        let Some(input_entity) = input_entity else {
            continue;
        };
        // Only run when the input's contents changed (cheap fast path).
        if inputs.get(input_entity).is_err() {
            continue;
        }
        let query = all_inputs
            .get(input_entity)
            .map(|c| c.get().to_lowercase())
            .unwrap_or_default();
        let mut any_visible = false;

        // For each list → each group → each item, toggle Display.
        for list_e in palette_kids.iter() {
            let Ok(list_kids) = lists.get(list_e) else {
                continue;
            };
            for group_e in list_kids.iter() {
                let Ok((mut group_node, group_kids)) = groups.get_mut(group_e) else {
                    continue;
                };
                let mut group_has_visible = false;
                for item_e in group_kids.iter() {
                    let Ok((item, mut item_node)) = items.get_mut(item_e) else {
                        continue;
                    };
                    let matches = query.is_empty() || item.search_text.contains(&query);
                    let new_display = if matches { Display::Flex } else { Display::None };
                    if item_node.display != new_display {
                        item_node.display = new_display;
                    }
                    if matches {
                        group_has_visible = true;
                    }
                }
                let new_display = if group_has_visible {
                    Display::Flex
                } else {
                    Display::None
                };
                if group_node.display != new_display {
                    group_node.display = new_display;
                }
                if group_has_visible {
                    any_visible = true;
                }
            }
            // Empty-state placeholder.
            for empty_e in list_kids.iter() {
                if let Ok(mut empty_node) = empty.get_mut(empty_e) {
                    let new_display = if any_visible {
                        Display::None
                    } else {
                        Display::Flex
                    };
                    if empty_node.display != new_display {
                        empty_node.display = new_display;
                    }
                }
            }
        }

        // If the previously-selected item just got hidden, reset
        // selection to the first visible item (or None).
        if let Ok(mut sel) = sels.get_mut(palette) {
            let still_visible = sel
                .selected
                .and_then(|e| items.get(e).ok())
                .map(|(_, n)| n.display != Display::None)
                .unwrap_or(false);
            if !still_visible {
                sel.selected = first_visible_item(palette_kids, &lists, &groups, &items);
            }
        }
    }
}

fn find_input_entity(
    palette_kids: &Children,
    input_rows: &Query<&Children, With<CommandInputRow>>,
    all_inputs: &Query<&TextInputContents>,
    children_q: &Query<&Children>,
) -> Option<Entity> {
    // Input lives at `palette → row → surround → inner` (`spawn_text_input`
    // gives us a surround/inner split). The inner is the only entity in
    // that subtree carrying `TextInputContents`, so walk the row's
    // descendants and return the first match.
    fn descend(
        e: Entity,
        children_q: &Query<&Children>,
        all_inputs: &Query<&TextInputContents>,
    ) -> Option<Entity> {
        if all_inputs.get(e).is_ok() {
            return Some(e);
        }
        let kids = children_q.get(e).ok()?;
        for child in kids.iter() {
            if let Some(found) = descend(child, children_q, all_inputs) {
                return Some(found);
            }
        }
        None
    }

    for child in palette_kids.iter() {
        if let Ok(row_kids) = input_rows.get(child) {
            for row_child in row_kids.iter() {
                if let Some(found) = descend(row_child, children_q, all_inputs) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn first_visible_item(
    palette_kids: &Children,
    lists: &Query<&Children, With<CommandList>>,
    groups: &Query<(&mut Node, &Children), (With<CommandGroup>, Without<CommandItem>, Without<CommandEmpty>)>,
    items: &Query<(&CommandItem, &mut Node), Without<CommandGroup>>,
) -> Option<Entity> {
    for list_e in palette_kids.iter() {
        let Ok(list_kids) = lists.get(list_e) else {
            continue;
        };
        for group_e in list_kids.iter() {
            let Ok((_, group_kids)) = groups.get(group_e) else {
                continue;
            };
            for item_e in group_kids.iter() {
                let Ok((item, node)) = items.get(item_e) else {
                    continue;
                };
                if node.display != Display::None && !item.disabled {
                    return Some(item_e);
                }
            }
        }
    }
    None
}

/// Paint items based on selection: selected item gets `accent` bg +
/// `accent_foreground` label text, others get transparent +
/// `foreground`. Cheap — runs every frame on the small item set.
pub(crate) fn repaint_items(
    palette: Res<ColorPalette>,
    palettes: Query<(&CommandSelection, &Children), With<Command>>,
    lists: Query<&Children, With<CommandList>>,
    groups: Query<&Children, With<CommandGroup>>,
    mut items: Query<(Entity, &CommandItem, &Interaction, &Children, &mut BackgroundColor)>,
    mut text_colors: Query<&mut TextColor>,
) {
    for (selection, palette_kids) in &palettes {
        for list_e in palette_kids.iter() {
            let Ok(list_kids) = lists.get(list_e) else {
                continue;
            };
            for group_e in list_kids.iter() {
                let Ok(group_kids) = groups.get(group_e) else {
                    continue;
                };
                for item_e in group_kids.iter() {
                    let Ok((entity, item, interaction, kids, mut bg)) = items.get_mut(item_e)
                    else {
                        continue;
                    };
                    let selected = selection.selected == Some(entity)
                        || matches!(interaction, Interaction::Hovered | Interaction::Pressed);
                    let (surface, text) = if item.disabled {
                        (
                            Color::NONE,
                            with_alpha(palette.foreground, 0.5),
                        )
                    } else if selected {
                        (palette.accent, palette.accent_foreground)
                    } else {
                        (Color::NONE, palette.foreground)
                    };
                    if bg.0 != surface {
                        *bg = BackgroundColor(surface);
                    }
                    // Update the first text child only (the label).
                    if let Some(label) = kids.iter().next()
                        && let Ok(mut tc) = text_colors.get_mut(label)
                    {
                        tc.0 = text;
                    }
                }
            }
        }
    }
}

/// Seed `InputFocus` to the palette root on spawn. Without this,
/// keyboard input goes nowhere because the palette is brand-new and
/// the click that opened it has already passed.
pub(crate) fn seed_focus_on_open(
    mut focus: ResMut<InputFocus>,
    palettes: Query<Entity, Added<Command>>,
) {
    if let Ok(palette) = palettes.single() {
        focus.set(palette, bevy::input_focus::FocusCause::Navigated);
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    let s = c.to_srgba();
    Color::srgba(s.red, s.green, s.blue, s.alpha * a)
}
