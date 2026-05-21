use bevy::prelude::*;

/// Root marker on a tabs row.
#[derive(Component, Default, Debug)]
pub struct Tabs;

/// Currently active tab index. Mutate this directly to programmatically
/// switch tabs — the paint system will move the indicator on next frame.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TabsActive(pub usize);

/// Marker on each trigger Button. Carries its index in the tabs row.
#[derive(Component, Clone, Copy, Debug)]
pub struct TabTrigger {
    pub index: usize,
}

/// Marker on the indicator Node that slides under the active trigger.
#[derive(Component, Default, Debug)]
pub struct TabIndicator;
