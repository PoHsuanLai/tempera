//! A small file tree built from `tree_row`s. Click a group header to
//! expand or collapse it.
//!
//! The example is also the argument for where the widget's
//! responsibilities stop: it owns the *hierarchy* (which rows exist at
//! which depth, and what a collapse hides) in about thirty lines below,
//! and tempera owns only the row. Swap the hierarchy for a layer list or
//! an outline and none of the widget changes.
//!
//! Pass `--screenshot <path>` to capture frame 120 and exit.

use std::collections::HashSet;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use tempera::prelude::*;

/// One entry of the example's tree. A real host would compute this from a
/// filesystem walk; the shape is what matters.
struct Entry {
    label: &'static str,
    /// `Some` when this row is a group, naming it.
    group: Option<&'static str>,
    /// The group this row sits inside, `None` at a root.
    parent: Option<&'static str>,
    suffix: Option<&'static str>,
}

const TREE: &[Entry] = &[
    Entry {
        label: "src",
        group: Some("src"),
        parent: None,
        suffix: None,
    },
    Entry {
        label: "main.rs",
        group: None,
        parent: Some("src"),
        suffix: Some("rs"),
    },
    Entry {
        label: "widgets",
        group: Some("widgets"),
        parent: Some("src"),
        suffix: None,
    },
    Entry {
        label: "button.rs",
        group: None,
        parent: Some("widgets"),
        suffix: Some("rs"),
    },
    Entry {
        label: "tree_row.rs",
        group: None,
        parent: Some("widgets"),
        suffix: Some("rs"),
    },
    Entry {
        label: "assets",
        group: Some("assets"),
        parent: None,
        suffix: None,
    },
    Entry {
        label: "Inter-Regular.otf",
        group: None,
        parent: Some("assets"),
        suffix: Some("otf"),
    },
    Entry {
        label: "Cargo.toml",
        group: None,
        parent: None,
        suffix: Some("toml"),
    },
];

/// Which groups are open. The host's state, not the widget's.
#[derive(Resource, Default)]
struct Expanded(HashSet<&'static str>);

/// Marker so a rebuild can clear the previous rows.
#[derive(Component)]
struct Row;

/// The list container.
#[derive(Component)]
struct RowList;

/// Set when the tree needs re-emitting.
#[derive(Resource, Default)]
struct Dirty(bool);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(TemperaPlugin)
        .init_resource::<Expanded>()
        .insert_resource(Dirty(true))
        .add_systems(Startup, setup)
        .add_systems(Update, rebuild);
    if let Some(path) = parse_screenshot_arg() {
        app.insert_resource(ScreenshotRequest {
            path,
            frames: 0,
            target: 120,
            captured: false,
        })
        .add_systems(Update, auto_screenshot);
    }
    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut font: ResMut<FontHandle>) {
    commands.spawn(Camera2d);
    font.regular = Some(asset_server.load("fonts/Inter-Regular.otf"));

    commands.spawn((
        RowList,
        Node {
            width: Val::Px(260.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::all(Val::Px(24.0)),
            row_gap: Val::Px(1.0),
            ..default()
        },
    ));
}

/// Emit the visible rows, deepest-first, skipping collapsed subtrees.
///
/// This is the part a tree-view crate would own. It is here to show the
/// seam, not because a widget should grow one.
fn rebuild(
    mut commands: Commands,
    mut dirty: ResMut<Dirty>,
    expanded: Res<Expanded>,
    style: TreeRowStyle,
    list: Query<Entity, With<RowList>>,
    rows: Query<Entity, With<Row>>,
) {
    if !dirty.0 || style.font.regular.is_none() {
        return;
    }
    let Ok(list) = list.single() else { return };
    dirty.0 = false;

    for row in &rows {
        commands.entity(row).despawn();
    }

    for node in TREE {
        let Some(depth) = visible_depth(node, &expanded.0) else {
            continue;
        };
        let mut spec = TreeRowSpec::new(node.label).depth(depth);
        if let Some(suffix) = node.suffix {
            spec = spec.suffix(suffix);
        }
        if let Some(group) = node.group {
            spec = spec.header().chevron(if expanded.0.contains(group) {
                ChevronState::Expanded
            } else {
                ChevronState::Collapsed
            });
        }

        let row = spawn_tree_row(&mut commands, &style, spec);
        commands.entity(row).insert((Row, ChildOf(list)));

        if let Some(group) = node.group {
            commands.entity(row).observe(
                move |_: On<Pointer<Click>>,
                      mut expanded: ResMut<Expanded>,
                      mut dirty: ResMut<Dirty>| {
                    if !expanded.0.remove(group) {
                        expanded.0.insert(group);
                    }
                    dirty.0 = true;
                },
            );
        }
    }
}

/// A node's depth, or `None` if an ancestor is collapsed.
fn visible_depth(node: &Entry, expanded: &HashSet<&'static str>) -> Option<u16> {
    let mut depth = 0;
    let mut parent = node.parent;
    while let Some(group) = parent {
        if !expanded.contains(group) {
            return None;
        }
        depth += 1;
        parent = TREE
            .iter()
            .find(|n| n.group == Some(group))
            .and_then(|n| n.parent);
    }
    Some(depth)
}

fn parse_screenshot_arg() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
}

#[derive(Resource)]
struct ScreenshotRequest {
    path: PathBuf,
    frames: u32,
    target: u32,
    captured: bool,
}

fn auto_screenshot(
    mut commands: Commands,
    mut req: ResMut<ScreenshotRequest>,
    mut exit: MessageWriter<AppExit>,
) {
    req.frames += 1;
    if req.frames == req.target && !req.captured {
        req.captured = true;
        let path = req.path.clone();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if req.captured && req.frames > req.target + 30 {
        exit.write(AppExit::Success);
    }
}
