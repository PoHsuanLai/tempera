//! Building the dialog, and keeping the sidebar in step with the tabs.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use tempera::dialog::{DialogConfig, DialogStyle, spawn_dialog};
use tempera::theme::{ColorPalette, FontHandle, Typography};

use crate::layout::SettingsLayout;
use crate::node::{
    SettingsBody, SettingsContentRow, SettingsDialog, SettingsOpen, SettingsSidebar, SidebarEntry,
};
use crate::tab::{ActiveTab, SettingsTab, TabBody, TabId, TabLabel, TabOrder};

/// The build runs here. A host that parents content into a tab body should
/// order itself `.after` this — though content that polls for its body by
/// id does not need to care.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SettingsBuildSet;

/// Theme and layout the build reads.
#[derive(SystemParam)]
pub struct SettingsStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    pub dialog: DialogStyle<'w>,
    pub layout: Res<'w, SettingsLayout>,
}

/// Optional close-button icon.
///
/// tempera ships no icon assets and its dialog has no text fallback for a
/// close button, so a host that wants one supplies the handle. Absent means
/// the dialog is dismissed by Escape or the backdrop, which tempera handles
/// either way.
#[derive(Resource, Debug, Clone, Default)]
pub struct SettingsCloseIcon(pub Option<Handle<Image>>);

/// The dialog title. A host renames it without forking the crate.
#[derive(Resource, Debug, Clone)]
pub struct SettingsTitle(pub String);

impl Default for SettingsTitle {
    fn default() -> Self {
        Self("Settings".to_string())
    }
}

/// The tabs a host has declared, in sidebar order.
///
/// Sorted by [`TabOrder`] then [`TabId`], so the order is a declaration
/// rather than an accident of which system ran first.
pub fn ordered_tabs(tabs: &Query<(&TabId, &TabLabel, &TabOrder), With<SettingsTab>>) -> Vec<TabId> {
    let mut all: Vec<(&TabOrder, &TabId)> = tabs.iter().map(|(id, _, order)| (order, id)).collect();
    all.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(b.1)));
    all.into_iter().map(|(_, id)| id.clone()).collect()
}

/// Spawn the dialog once, then keep its sidebar and bodies matching the
/// declared tabs.
///
/// One system rather than a spawn-once plus a reconcile, because they are
/// the same code path: a tab declared on frame 1 and a tab declared by an
/// extension on frame 900 must produce the same sidebar entry and the same
/// body.
pub(crate) fn build_settings(
    mut commands: Commands,
    style: SettingsStyle,
    title: Res<SettingsTitle>,
    close_icon: Res<SettingsCloseIcon>,
    tabs: Query<(&TabId, &TabLabel, &TabOrder), With<SettingsTab>>,
    dialogs: Query<(Entity, &ActiveTab), With<SettingsDialog>>,
    sidebars: Query<Entity, With<SettingsSidebar>>,
    bodies: Query<(Entity, &TabId), With<TabBody>>,
    entries: Query<(Entity, &SidebarEntry)>,
    body_parents: Query<Entity, With<SettingsBody>>,
) {
    let ordered = ordered_tabs(&tabs);

    // Spawn the shell once.
    if dialogs.is_empty() {
        if style.font.regular.is_none() {
            // Rows spawned before the font loads render with no glyphs and
            // never re-measure. Wait.
            return;
        }
        spawn_shell(&mut commands, &style, &title, &close_icon);
        return;
    }

    let Ok(sidebar) = sidebars.single() else {
        return;
    };
    let Ok(body_parent) = body_parents.single() else {
        return;
    };

    // Add an entry + body for each declared tab that has neither, and drop
    // both for any that is no longer declared.
    let have_entry: Vec<TabId> = entries.iter().map(|(_, e)| e.0.clone()).collect();
    let have_body: Vec<TabId> = bodies.iter().map(|(_, id)| id.clone()).collect();

    for id in &ordered {
        if !have_entry.contains(id) {
            let label = tabs
                .iter()
                .find(|(tab_id, _, _)| *tab_id == id)
                .map(|(_, label, _)| label.0.clone())
                .unwrap_or_default();
            spawn_sidebar_entry(&mut commands, &style, sidebar, id.clone(), label);
        }
        if !have_body.contains(id) {
            commands.spawn((
                TabBody,
                id.clone(),
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::vertical(Val::Px(8.0)),
                    display: Display::None,
                    ..default()
                },
                ChildOf(body_parent),
            ));
        }
    }

    for (entity, entry) in &entries {
        if !ordered.contains(&entry.0) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, id) in &bodies {
        if !ordered.contains(id) {
            // Despawns the host's content with it — the same contract
            // `shellie-dock` gives a pane dropped from the layout.
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_shell(
    commands: &mut Commands,
    style: &SettingsStyle,
    title: &SettingsTitle,
    close_icon: &SettingsCloseIcon,
) {
    let layout = &style.layout;
    let mut config = DialogConfig::new()
        .title(title.0.clone())
        .size(layout.width, layout.height);
    if let Some(icon) = &close_icon.0 {
        config = config.closable_with_icon(icon.clone());
    }
    let parts = spawn_dialog(commands, &style.dialog, config);

    commands
        .entity(parts.root)
        .insert((SettingsDialog, ActiveTab::none(), SettingsOpen(false)));

    // `flex_grow` rather than a hand-computed height: the content row takes
    // whatever the title bar leaves, so this crate never has to know how
    // tall tempera draws it.
    let row = commands
        .spawn((
            SettingsContentRow,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                overflow: Overflow::clip(),
                ..default()
            },
            ChildOf(parts.content),
        ))
        .id();

    commands.spawn((
        SettingsSidebar,
        Node {
            width: Val::Px(layout.sidebar_width),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(12.0)),
            row_gap: Val::Px(2.0),
            border: UiRect::right(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(style.palette.border),
        BackgroundColor(style.palette.background),
        ChildOf(row),
    ));

    commands.spawn((
        SettingsBody,
        Node {
            flex_grow: 1.0,
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ChildOf(row),
    ));
}

fn spawn_sidebar_entry(
    commands: &mut Commands,
    style: &SettingsStyle,
    sidebar: Entity,
    id: TabId,
    label: String,
) {
    let click_id = id.clone();
    let entry = commands
        .spawn((
            SidebarEntry(id),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Interaction::default(),
            ChildOf(sidebar),
        ))
        .observe(
            move |on: On<Pointer<Click>>,
                  parents: Query<&ChildOf>,
                  mut dialogs: Query<&mut ActiveTab>| {
                // Walk up to *this entry's own* dialog. Writing every
                // `ActiveTab` would move the selection in a second settings
                // dialog too — the bug putting state on the entity rather
                // than in a resource exists to prevent.
                let mut cursor = on.event_target();
                loop {
                    if let Ok(mut active) = dialogs.get_mut(cursor) {
                        active.set(click_id.clone());
                        return;
                    }
                    let Ok(parent) = parents.get(cursor) else {
                        return;
                    };
                    cursor = parent.parent();
                }
            },
        )
        .id();

    commands.spawn((
        Text::new(label),
        style.font.text_font(style.typography.sm),
        TextColor(style.palette.muted_foreground),
        ChildOf(entry),
        bevy::picking::Pickable::IGNORE,
    ));
}
