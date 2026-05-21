use bevy::prelude::*;
use bevy::ui::ComputedNode;
use bevy::ui_widgets::Activate;

use super::components::{TabIndicator, TabTrigger, Tabs, TabsActive};
use super::spawn::INDICATOR_INSET;
use crate::theme::ColorPalette;

/// Emitted when a tab is activated. Listen on the tabs root entity:
/// `commands.entity(tabs).observe(|on: On<TabsChanged>| ...)`.
#[derive(bevy::ecs::event::EntityEvent, Clone, Copy, Debug)]
pub struct TabsChanged {
    #[event_target]
    pub tabs: Entity,
    pub active: usize,
}

/// Translate `Activate` on a `TabTrigger` into a `TabsChanged` event
/// and update the parent's `TabsActive`.
pub(crate) fn trigger_on_activate(
    on: On<Activate>,
    triggers: Query<(&TabTrigger, &ChildOf)>,
    mut tabs: Query<&mut TabsActive, With<Tabs>>,
    mut commands: Commands,
) {
    let Ok((trigger, parent)) = triggers.get(on.entity) else {
        return;
    };
    let Ok(mut active) = tabs.get_mut(parent.0) else {
        return;
    };
    if active.0 != trigger.index {
        active.0 = trigger.index;
        commands.trigger(TabsChanged {
            tabs: parent.0,
            active: trigger.index,
        });
    }
}

/// Retint trigger text on selection / hover changes.
pub(crate) fn repaint_triggers(
    palette: Res<ColorPalette>,
    tabs: Query<(&TabsActive, &Children), With<Tabs>>,
    tabs_changed: Query<(&TabsActive, &Children), (With<Tabs>, Changed<TabsActive>)>,
    triggers: Query<
        (Entity, &TabTrigger, &Interaction, &Children, &ChildOf),
        Or<(Changed<Interaction>, Added<TabTrigger>)>,
    >,
    all_triggers: Query<(&TabTrigger, &Interaction, &Children), With<TabTrigger>>,
    mut texts: Query<&mut TextColor>,
) {
    // Repaint triggers whose Interaction just changed.
    let mut handled: Vec<Entity> = Vec::new();
    for (entity, trigger, interaction, kids, parent) in &triggers {
        handled.push(entity);
        let Ok((active, _)) = tabs.get(parent.0) else {
            continue;
        };
        let color = trigger_text_color(&palette, trigger.index == active.0, interaction);
        for child in kids.iter() {
            if let Ok(mut tc) = texts.get_mut(child) {
                *tc = TextColor(color);
            }
        }
    }

    // When `TabsActive` flips, repaint *every* trigger in that row
    // so the previously-active one dims. Cheap because tabs rows are
    // short.
    for (active, kids) in &tabs_changed {
        for child in kids.iter() {
            if handled.contains(&child) {
                continue;
            }
            let Ok((trigger, interaction, text_kids)) = all_triggers.get(child) else {
                continue;
            };
            let color = trigger_text_color(&palette, trigger.index == active.0, interaction);
            for grand in text_kids.iter() {
                if let Ok(mut tc) = texts.get_mut(grand) {
                    *tc = TextColor(color);
                }
            }
        }
    }
}

fn trigger_text_color(palette: &ColorPalette, is_active: bool, interaction: &Interaction) -> Color {
    if is_active {
        return palette.foreground;
    }
    match interaction {
        Interaction::Hovered | Interaction::Pressed => palette.foreground,
        Interaction::None => palette.muted_foreground,
    }
}

/// Move the indicator to the active trigger's measured bounds.
/// Lerps to the target each frame for a smooth slide.
///
/// `ComputedNode::size` returns **physical** pixels (per its docstring,
/// and confirmed empirically: a 113-logical-px trigger reports
/// size=226 on a 2× retina display). We position the indicator with
/// `Val::Px(...)` which is logical, so multiply by
/// `inverse_scale_factor` to convert. Without this, the indicator
/// renders at 2× its intended width and covers the active trigger
/// plus its neighbours.
pub(crate) fn move_indicator(
    time: Res<Time>,
    tabs: Query<(&TabsActive, &Children), With<Tabs>>,
    triggers: Query<(&TabTrigger, &ComputedNode, &Node)>,
    mut indicator: Query<(&mut Node, &ChildOf), (With<TabIndicator>, Without<TabTrigger>)>,
) {
    for (mut node, parent) in &mut indicator {
        let Ok((active, kids)) = tabs.get(parent.0) else {
            continue;
        };

        let mut target_left = INDICATOR_INSET;
        let mut target_width = 0.0;
        let mut found = false;
        for child in kids.iter() {
            let Ok((trigger, computed, _)) = triggers.get(child) else {
                continue;
            };
            let logical_width = computed.size().x * computed.inverse_scale_factor();
            if trigger.index == active.0 {
                target_width = logical_width;
                found = true;
            } else if !found {
                target_left += logical_width;
            }
        }

        let alpha = (time.delta_secs() * 12.0).min(1.0);
        let cur_left = match node.left {
            Val::Px(v) => v,
            _ => target_left,
        };
        let cur_width = match node.width {
            Val::Px(v) => v,
            _ => target_width,
        };
        node.left = Val::Px(cur_left + (target_left - cur_left) * alpha);
        node.width = Val::Px(cur_width + (target_width - cur_width) * alpha);
    }
}
