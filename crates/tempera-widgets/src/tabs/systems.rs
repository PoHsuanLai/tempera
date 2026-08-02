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

/// Retint trigger text on selection / hover / palette changes.
///
/// One unfiltered pass. There used to be a `Changed<Interaction>` query, a
/// `Changed<TabsActive>` query and a `handled` vec to stop them
/// double-painting — the second existed because when the active tab flips,
/// the *previously* active trigger has to dim, and that entity's own
/// components did not change. A palette swap is the same shape, one level
/// wider. Both are in the run condition now, and the write compares first.
pub(crate) fn repaint_triggers(
    palette: Res<ColorPalette>,
    tabs: Query<(&TabsActive, &Children), With<Tabs>>,
    triggers: Query<(&TabTrigger, &Interaction, &Children, &ChildOf), With<TabTrigger>>,
    mut texts: Query<&mut TextColor>,
) {
    for (trigger, interaction, kids, parent) in &triggers {
        let Ok((active, _)) = tabs.get(parent.0) else {
            continue;
        };
        let color = trigger_text_color(&palette, trigger.index == active.0, interaction);
        for child in kids.iter() {
            if let Ok(mut tc) = texts.get_mut(child)
                && tc.0 != color
            {
                *tc = TextColor(color);
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
