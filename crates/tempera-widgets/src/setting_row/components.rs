use bevy::prelude::*;

/// Root marker on one row of a settings form.
///
/// Behaviour-free and paint-free: unlike [`list_row`](crate::list_row) this
/// widget has no hover state and registers no systems. A form row is read
/// and edited through its control, never selected or activated as a whole.
#[derive(Component, Default, Debug)]
pub struct SettingRow;

/// Marker on the row's label `Text`.
#[derive(Component, Default, Debug)]
pub struct SettingRowLabel;

/// Marker on the row's description `Text` — the second, dimmer line.
#[derive(Component, Default, Debug)]
pub struct SettingRowDescription;

/// The row's control slot. **Parent your widget here.**
///
/// Fixed-width, deliberately: a settings form reads as a column of aligned
/// controls, and a slot that grew to fit its content would ragged that
/// edge. This is the opposite choice from
/// [`ListRowTrail`](crate::list_row::ListRowTrail), which uses a floor
/// precisely because its contents are unpredictable.
#[derive(Component, Default, Debug)]
pub struct SettingRowControl;

/// Marker on a section heading's `Text`.
///
/// The heading node carries [`SettingSection`]; this is the text inside it.
/// Two markers rather than one plus a child-walk, so the repaint system can
/// name what it writes instead of re-deriving which child is the label.
#[derive(Component, Default, Debug)]
pub struct SettingSectionLabel;

/// Marker on a section heading spawned by
/// [`spawn_section_header`](super::spawn_section_header).
#[derive(Component, Default, Debug)]
pub struct SettingSection;
