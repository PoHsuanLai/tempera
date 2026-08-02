use bevy::prelude::*;

use crate::context_menu::MenuItemSpec;

/// Marker on a dropdown-menu trigger Button. Carries the items the
/// menu will spawn on click.
#[derive(Component, Debug, Clone)]
pub struct DropdownTrigger {
    pub items: Vec<MenuItemSpec>,
}
