use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::ui_widgets::Activate;

use super::components::DropdownTrigger;
use crate::context_menu::{MenuRequest, OpenContextMenu};

/// On `Activate` of a [`DropdownTrigger`], compute its window-space
/// bottom-left in logical pixels and write an `OpenContextMenu`
/// request. UI nodes carry [`UiGlobalTransform`], not the regular
/// [`GlobalTransform`] — querying for the wrong one silently filters
/// the entity out and the menu never opens.
///
/// Both `UiGlobalTransform.translation` and `ComputedNode::size` are
/// in **physical** pixels. The popup positions itself with
/// `Val::Px(...)` (logical), so we multiply by `inverse_scale_factor`.
pub(crate) fn open_on_trigger_activate(
    on: On<Activate>,
    triggers: Query<(&DropdownTrigger, &ComputedNode, &UiGlobalTransform)>,
    mut writer: MessageWriter<OpenContextMenu>,
) {
    let Ok((trigger, node, transform)) = triggers.get(on.entity) else {
        return;
    };
    let scale = node.inverse_scale_factor();
    let center = transform.translation * scale;
    let half = node.size() * scale * 0.5;
    let anchor = Vec2::new(center.x - half.x, center.y + half.y + 4.0);

    writer.write(OpenContextMenu(MenuRequest {
        anchor,
        items: trigger.items.clone(),
    }));
}
