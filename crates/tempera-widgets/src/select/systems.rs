use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};

use super::components::{
    Select, SelectChevron, SelectDisplayText, SelectOptions, SelectValue, ValueChange,
};
use crate::context_menu::{MenuItemActivated, MenuItemSpec, MenuRequest, OpenContextMenu};

/// On click of a Select trigger, build MenuItemSpecs from its
/// options and open a context menu popup anchored below the trigger.
pub(crate) fn open_on_click(
    on: On<Pointer<Click>>,
    selects: Query<
        (
            Entity,
            &SelectOptions,
            &SelectValue,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        With<Select>,
    >,
    mut writer: MessageWriter<OpenContextMenu>,
) {
    let Ok((entity, options, current, node, transform)) = selects.get(on.entity) else {
        return;
    };

    let scale = node.inverse_scale_factor();
    let center = transform.translation * scale;
    let half = node.size() * scale * 0.5;
    let anchor = Vec2::new(center.x - half.x, center.y + half.y + 2.0);

    let items: Vec<MenuItemSpec> = options
        .0
        .iter()
        .map(|opt| {
            let mut spec = MenuItemSpec::new(&opt.id).label(if opt.id == current.0 {
                format!("✓ {}", opt.label)
            } else {
                format!("   {}", opt.label)
            });
            spec = spec.origin(entity);
            spec
        })
        .collect();

    writer.write(OpenContextMenu(MenuRequest { anchor, items }));
}

/// When a menu item is activated and its origin points to a Select
/// entity, update the SelectValue and display text, then trigger
/// ValueChange.
pub(crate) fn on_menu_item_activated(
    mut events: MessageReader<MenuItemActivated>,
    mut selects: Query<(&mut SelectValue, &SelectOptions, &Children), With<Select>>,
    mut texts: Query<&mut Text, With<SelectDisplayText>>,
    mut commands: Commands,
) {
    for ev in events.read() {
        let Some(origin) = ev.entity else {
            continue;
        };
        let Ok((mut value, options, children)) = selects.get_mut(origin) else {
            continue;
        };
        if value.0 == ev.id {
            continue;
        }
        let label = options
            .0
            .iter()
            .find(|o| o.id == ev.id)
            .map(|o| o.label.clone())
            .unwrap_or_default();

        value.0 = ev.id.clone();

        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                if **text != label {
                    **text = label.clone();
                }
            }
        }

        commands.trigger(ValueChange {
            source: origin,
            value: ev.id.clone(),
            label,
        });
    }
}

/// Hover bg tint on the select trigger.
///
/// Reads [`MenuTokens`] rather than the two hardcoded white alphas it used
/// to hold: those were a copy *by value* of `item_hover_bg` and
/// `item_active_bg`, so a recoloured theme could never have reached this
/// widget even once the gating was fixed. The select's trigger is a menu
/// surface, so it takes the menu's colours.
pub(crate) fn paint_select_hover(
    menu: Res<crate::menu_tokens::MenuTokens>,
    palette: Res<crate::theme::ColorPalette>,
    mut selects: Query<(&Interaction, &mut BackgroundColor, &mut BorderColor), With<Select>>,
    mut display: Query<&mut TextColor, (With<SelectDisplayText>, Without<SelectChevron>)>,
    mut chevrons: Query<&mut TextColor, With<SelectChevron>>,
) {
    // The border and the two text nodes are painted once at spawn, so before
    // this they kept the old theme's colours while the fill above followed
    // `MenuTokens`. The display text takes `foreground`, which *inverts*
    // between palettes — in a light theme it went white-on-white and the
    // select looked empty rather than unthemed.
    let border_want = BorderColor::all(palette.border);
    for mut color in &mut display {
        if color.0 != palette.foreground {
            color.0 = palette.foreground;
        }
    }
    for mut color in &mut chevrons {
        if color.0 != palette.muted_foreground {
            color.0 = palette.muted_foreground;
        }
    }

    let resting = menu.item_hover_bg;
    let hovered = menu.item_active_bg;
    for (interaction, mut bg, mut border) in &mut selects {
        let target = match interaction {
            Interaction::Hovered | Interaction::Pressed => hovered,
            Interaction::None => resting,
        };
        if bg.0 != target {
            bg.0 = target;
        }
        if *border != border_want {
            *border = border_want;
        }
    }
}
