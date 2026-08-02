use bevy::prelude::*;

/// Marker on the select root entity.
#[derive(Component, Debug)]
pub struct Select;

/// The current selected value id. Updated by the activation system.
#[derive(Component, Debug, Clone)]
pub struct SelectValue(pub String);

/// The available options. Each entry is `(id, label)`.
#[derive(Component, Debug, Clone)]
pub struct SelectOptions(pub Vec<SelectOption>);

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub id: String,
    pub label: String,
}

/// Marker on the text child that displays the current selection.
#[derive(Component, Debug)]
pub struct SelectDisplayText;

/// Marker on the chevron icon child.
#[derive(Component, Debug)]
pub struct SelectChevron;

/// Fired on the select root when the user picks a new value.
#[derive(EntityEvent, Debug, Clone)]
pub struct ValueChange {
    #[event_target]
    pub source: Entity,
    pub value: String,
    pub label: String,
}
