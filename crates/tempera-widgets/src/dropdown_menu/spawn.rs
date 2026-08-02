use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::DropdownTrigger;
use crate::button::{spawn_button, ButtonContent, ButtonStyle, ButtonVariant};
use crate::context_menu::MenuItemSpec;

#[derive(SystemParam)]
pub struct DropdownStyle<'w> {
    pub button: ButtonStyle<'w>,
}

/// Spawn a dropdown-menu trigger. Returns the entity (a styled
/// [`crate::button::Button`]). Clicking or activating it opens a
/// tempera context-menu popup anchored beneath the trigger.
pub fn spawn_dropdown(
    commands: &mut Commands,
    style: &DropdownStyle,
    label: impl Into<String>,
    items: Vec<MenuItemSpec>,
) -> Entity {
    let id = spawn_button(
        commands,
        &style.button,
        ButtonContent::text(label),
        ButtonVariant::Ghost,
    );
    commands.entity(id).insert(DropdownTrigger { items });
    id
}
