//! SVG icons, behind the `svg` feature.
//!
//! Every tempera widget that shows an icon takes an `Option<Handle<Image>>`
//! and never produces one — the host brings its own. If the host ships
//! pre-rendered PNGs it needs nothing from this module; `bevy`'s own image
//! loaders already cover it. This exists for the case where the icons are
//! SVGs, which is common enough (and is what extension-supplied icons look
//! like) to be worth an answer.
//!
//! The answer is [`bevy_resvg`], re-exported. Tempera adds one thing on top:
//! [`SvgIcons`], which turns a loaded SVG into the `Handle<Image>` the widget
//! APIs already speak.
//!
//! # Why not `UiSvg`
//!
//! `bevy_resvg` ships a `UiSvg` component that inserts an `ImageNode` once
//! the asset loads. That is the right shape for a bare `Node`, and the wrong
//! shape here: its query is filtered `Without<ImageNode>`, and every tempera
//! widget builds its own `ImageNode` at spawn from the handle it was given.
//! Handing `UiSvg` to a widget produces an entity the plugin will not touch.
//!
//! So this module goes the other way. `SvgFile` holds a public `Image`, so a
//! handle can be lifted out of it and passed to `spawn_button`,
//! `TreeRowSpec::icon`, `DialogConfig::closable_with_icon` and the rest
//! unchanged.
//!
//! # Loading is not instant
//!
//! `asset_server.load` returns before the file is read, and rasterising
//! happens in the loader. So the `Handle<Image>` does not exist on the frame
//! you ask for it. That is the price of lazy loading, and it is the right
//! price — the alternative is rasterising every icon at startup whether or
//! not it is ever shown.
//!
//! [`SvgIcons`] holds the pending `SvgFile` handles and resolves them as they
//! arrive; widgets spawned against a not-yet-ready icon simply have no glyph
//! for a frame or two.
//!
//! ```ignore
//! app.add_plugins(TemperaSvgPlugin);
//!
//! fn spawn_ui(mut commands: Commands, assets: Res<AssetServer>, mut icons: ResMut<SvgIcons>) {
//!     let search = icons.load(&assets, "icons/search.svg");
//!     // ... later, once loaded:
//!     if let Some(handle) = icons.get("icons/search.svg") {
//!         spawn_button(&mut commands, &style, ButtonContent::icon(handle));
//!     }
//! }
//! ```

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub use bevy_resvg::prelude::{SvgFile, SvgFileLoaderSettings, TargetRenderSize};

/// Registers `bevy_resvg`'s asset loader plus [`SvgIcons`].
///
/// Idempotent in the same way tempera's other plugins are: adding it twice,
/// or adding it alongside a host that already registered `SvgPlugin`, does
/// not panic.
pub struct TemperaSvgPlugin;

impl Plugin for TemperaSvgPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_resvg::plugin::SvgPlugin>() {
            app.add_plugins(bevy_resvg::plugin::SvgPlugin);
        }
        app.init_resource::<SvgIcons>()
            .add_systems(Update, resolve_loaded_icons);
    }
}

/// SVG icons keyed by asset path, resolved to `Handle<Image>` once loaded.
///
/// Keyed by path rather than by a caller-chosen id: the path is already the
/// icon's identity to the asset server, and a second naming layer would only
/// be somewhere for the two to disagree. A host that wants short names can
/// keep its own `&'static str -> path` table — that table is *its* icon set,
/// not tempera's business.
#[derive(Resource, Default)]
pub struct SvgIcons {
    /// Loaded and converted. This is what [`Self::get`] answers from.
    ready: HashMap<String, Handle<Image>>,
    /// In flight. The `SvgFile` handle is held so the asset is not dropped
    /// before the load completes.
    pending: HashMap<String, Handle<SvgFile>>,
}

impl SvgIcons {
    /// Begin loading `path` as an icon. Returns immediately; the image
    /// becomes available from [`Self::get`] a frame or more later.
    ///
    /// Calling this again for a path already loading or loaded is a no-op, so
    /// it is safe to call from a system that runs every frame.
    pub fn load(&mut self, assets: &AssetServer, path: impl Into<String>) {
        let path = path.into();
        if self.ready.contains_key(&path) || self.pending.contains_key(&path) {
            return;
        }
        let handle: Handle<SvgFile> = assets.load(path.clone());
        self.pending.insert(path, handle);
    }

    /// The rasterised image for `path`, once it has loaded.
    pub fn get(&self, path: &str) -> Option<&Handle<Image>> {
        self.ready.get(path)
    }

    /// Whether every requested icon has finished loading. Useful as a run
    /// condition for a spawn system that wants all its glyphs at once.
    pub fn all_ready(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Move icons from `pending` to `ready` as their `SvgFile` assets land.
fn resolve_loaded_icons(
    mut icons: ResMut<SvgIcons>,
    svg_files: Res<Assets<SvgFile>>,
    mut images: ResMut<Assets<Image>>,
) {
    if icons.pending.is_empty() {
        return;
    }
    let mut resolved: Vec<(String, Handle<Image>)> = Vec::new();
    for (path, handle) in &icons.pending {
        if let Some(file) = svg_files.get(handle.id()) {
            resolved.push((path.clone(), images.add(file.0.clone())));
        }
    }
    for (path, image) in resolved {
        icons.pending.remove(&path);
        icons.ready.insert(path, image);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Re-requesting an icon that already loaded does not put it back in
    /// flight.
    ///
    /// This is the property that makes `load` safe to call from a system
    /// running every frame — the shape a host reaches for first. Without the
    /// guard a settled icon returns to `pending` on the very next frame,
    /// `all_ready` never latches, and `resolve_loaded_icons` adds a fresh
    /// `Image` to `Assets` every frame forever.
    ///
    /// Note what this test deliberately does *not* do: count `pending` after
    /// two `load` calls for the same path. `pending` is keyed by path, so the
    /// second insert overwrites the first and the count is identical with or
    /// without the guard. An earlier version asserted exactly that, and
    /// passed with the guard deleted.
    #[test]
    fn re_requesting_a_loaded_icon_does_not_reload_it() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<SvgFile>()
            .init_asset::<Image>()
            .init_resource::<SvgIcons>()
            .add_systems(Update, resolve_loaded_icons);

        // Stand in for a completed load.
        let world = app.world_mut();
        let handle = world
            .resource_mut::<Assets<SvgFile>>()
            .add(SvgFile(Image::default()));
        world
            .resource_mut::<SvgIcons>()
            .pending
            .insert("a.svg".to_string(), handle);
        app.update();
        assert!(app.world().resource::<SvgIcons>().all_ready());

        // Ask again for the icon that is already resolved.
        let world = app.world_mut();
        let assets = world.resource::<AssetServer>().clone();
        world.resource_mut::<SvgIcons>().load(&assets, "a.svg");

        let icons = app.world().resource::<SvgIcons>();
        assert!(
            icons.pending.is_empty(),
            "a loaded icon was queued for loading again"
        );
        assert!(icons.all_ready());
    }

    /// An icon that has not loaded yet reads as absent rather than as a
    /// default handle. A default `Handle<Image>` renders as a white square,
    /// so answering with one would put a visible artifact on screen instead
    /// of nothing.
    #[test]
    fn an_unloaded_icon_is_absent() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<SvgFile>()
            .init_asset::<Image>()
            .init_resource::<SvgIcons>();

        let world = app.world_mut();
        let assets = world.resource::<AssetServer>().clone();
        world.resource_mut::<SvgIcons>().load(&assets, "a.svg");
        assert!(world.resource::<SvgIcons>().get("a.svg").is_none());
        assert!(!world.resource::<SvgIcons>().all_ready());
    }

    /// A loaded `SvgFile` becomes a `Handle<Image>` and leaves `pending`.
    ///
    /// The asset is inserted directly rather than read off disk, so the test
    /// exercises tempera's resolve step without depending on `bevy_resvg`'s
    /// rasteriser or on an async load completing within the test.
    #[test]
    fn a_loaded_svg_becomes_an_image() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<SvgFile>()
            .init_asset::<Image>()
            .init_resource::<SvgIcons>()
            .add_systems(Update, resolve_loaded_icons);

        // Stand in for a completed load: put an `SvgFile` in `Assets` and
        // register its handle as pending under a path.
        let world = app.world_mut();
        let handle = world
            .resource_mut::<Assets<SvgFile>>()
            .add(SvgFile(Image::default()));
        world
            .resource_mut::<SvgIcons>()
            .pending
            .insert("a.svg".to_string(), handle);

        app.update();

        let icons = app.world().resource::<SvgIcons>();
        assert!(icons.get("a.svg").is_some(), "icon did not resolve");
        assert!(icons.all_ready());
    }
}
