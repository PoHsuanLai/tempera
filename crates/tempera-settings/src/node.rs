//! The live dialog, as components.

use bevy::prelude::*;

use crate::tab::TabId;

/// Marker on the settings dialog root — the entity tempera's
/// `spawn_dialog` hands back, carrying [`ActiveTab`](crate::ActiveTab).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsDialog;

/// The row holding the sidebar and the body.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsContentRow;

/// The sidebar column of tab entries.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsSidebar;

/// One clickable sidebar entry, naming the tab it selects.
#[derive(Component, Debug, Clone)]
pub struct SidebarEntry(pub TabId);

/// The scrollable column that tab bodies live in.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SettingsBody;

/// Whether the dialog is showing.
///
/// A component on the dialog root rather than a resource, so a host with
/// two dialogs can open them independently — and so "is it open" is read
/// off the same entity that carries which tab is active.
///
/// This crate never sets it. Opening is a host decision (a menu item, a
/// keybind, a command), and closing is reported by tempera's
/// `DialogDismissed`, which the host maps back. See the crate docs on why
/// the flag is not swallowed here.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettingsOpen(pub bool);
