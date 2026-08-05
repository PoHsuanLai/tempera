//! The plugin, and the one piece of configuration tempera needs from its host.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::binding::{apply_saved_bindings, strip_unbound_keybinds};
use crate::command::{CommandRegistry, unregister_despawned_commands};
use crate::dispatch::{HeldClaims, dispatch_commands, drain_held_claims, window_lost_focus};
use crate::persist::SavedKeybinds;

/// Where this application's keybinds are read from and written to.
///
/// Inserted by [`TemperaInputPlugin`] so that a host system which rebinds a
/// command can write the file back without having to re-derive the path — and
/// without tempera having to guess where it should be.
///
/// This replaces an `AppName(String)`. A name is not what a writer needs: it
/// needed the *path*, and had to reconstruct it through
/// [`SavedKeybinds::storage_path`] every time. Naming the thing the consumer
/// actually uses also makes the resource the single owner of that answer, so a
/// host that overrides the path has overridden it everywhere.
#[derive(Resource, Clone, Debug)]
pub struct KeybindsPath(pub PathBuf);

/// Keybinds and commands.
///
/// ```
/// use bevy::prelude::*;
/// use tempera_input::plugin::TemperaInputPlugin;
///
/// let mut app = App::new();
/// app.add_plugins(TemperaInputPlugin::new("my-app"));
/// ```
///
/// [`new`](Self::new) takes an application name and puts the file in the
/// conventional place. A host that needs somewhere else — a portable install,
/// a `--config` flag, a test that must not touch the developer's home
/// directory — uses [`at`](Self::at) instead.
pub struct TemperaInputPlugin {
    keybinds_path: PathBuf,
}

impl TemperaInputPlugin {
    /// Keybinds in the conventional location for `app_name`:
    /// `<config dir>/<app_name>/keybinds.json`.
    pub fn new(app_name: impl AsRef<str>) -> Self {
        Self {
            keybinds_path: SavedKeybinds::storage_path(app_name.as_ref()),
        }
    }

    /// Keybinds at an explicit path.
    ///
    /// The escape hatch from the convention. Without it a downstream test
    /// cannot isolate itself from the developer's real config file, which is
    /// the concrete problem that motivated taking a path at all.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            keybinds_path: path.into(),
        }
    }
}

impl Plugin for TemperaInputPlugin {
    fn build(&self, app: &mut App) {
        let saved = SavedKeybinds::load(&self.keybinds_path);

        app.insert_resource(KeybindsPath(self.keybinds_path.clone()))
            .insert_resource(saved)
            .init_resource::<CommandRegistry>()
            .init_resource::<HeldClaims>()
            // Registry upkeep is an observer rather than a system so a
            // despawned command's id is freed at the despawn, not a frame
            // later — otherwise a remove-then-re-add in one frame trips the
            // duplicate-id guard.
            .add_observer(unregister_despawned_commands)
            // Overrides land in `PostStartup`, after every plugin has had its
            // `Startup` chance to register commands — an override applied
            // before its command exists would be silently lost.
            .add_systems(
                PostStartup,
                (apply_saved_bindings, strip_unbound_keybinds).chain(),
            )
            .add_message::<crate::capture::ChordCaptured>()
            // Before dispatch, and dispatch is suppressed while it runs.
            // Recording `Cmd+S` must not *also* save: the recorder and the
            // dispatcher read the same `ButtonInput`, so without this a user
            // rebinding a shortcut fires whatever that shortcut currently
            // does, every time they try to change it.
            .add_systems(
                Update,
                crate::capture::capture_chord
                    .run_if(resource_exists::<crate::capture::ChordCapture>)
                    .before(CommandDispatch),
            )
            .add_systems(
                Update,
                dispatch_commands
                    .in_set(CommandDispatch)
                    .run_if(not(resource_exists::<crate::capture::ChordCapture>)),
            )
            // Losing the window is the one way a held command can miss its
            // release: the key comes up while another app has focus, so we
            // never see it. Drain instead of stranding the claim.
            .add_systems(
                Update,
                drain_held_claims
                    .run_if(window_lost_focus)
                    .after(CommandDispatch),
            );
    }
}

/// The set [`dispatch_commands`] runs in, so an app can order its own systems
/// around command handling.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandDispatch;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::command::{AppCommandExt, CommandLabel, CommandRegistry, cmd, on_press};
    use crate::persist::scratch_keybinds_path;

    struct Undo;
    impl Command for Undo {
        const ID: &'static str = "edit.undo";
    }

    #[test]
    fn registry_upkeep_is_wired_by_the_plugin() {
        let mut app = App::new();
        app.add_plugins(TemperaInputPlugin::at(scratch_keybinds_path()));
        app.spawn_command(cmd::<Undo>((CommandLabel::new("Undo"), on_press(|_| {}))));

        let entity = app
            .world()
            .resource::<CommandRegistry>()
            .get("edit.undo")
            .expect("registered");

        app.world_mut().entity_mut(entity).despawn();

        assert!(
            app.world()
                .resource::<CommandRegistry>()
                .get("edit.undo")
                .is_none(),
            "the plugin must wire the despawn observer"
        );
    }

    #[test]
    fn a_command_can_be_replaced_within_one_frame() {
        // Re-registering after a despawn is what a reloading extension does.
        // If id release were deferred, the re-add would hit the duplicate
        // guard and the command would vanish.
        let mut app = App::new();
        app.add_plugins(TemperaInputPlugin::at(scratch_keybinds_path()));
        app.spawn_command(cmd::<Undo>((CommandLabel::new("v1"), on_press(|_| {}))));

        let first = app
            .world()
            .resource::<CommandRegistry>()
            .get("edit.undo")
            .unwrap();
        app.world_mut().entity_mut(first).despawn();

        app.spawn_command(cmd::<Undo>((CommandLabel::new("v2"), on_press(|_| {}))));

        let second = app.world().resource::<CommandRegistry>().get("edit.undo");
        assert!(second.is_some(), "re-registration must succeed");
        assert_ne!(second, Some(first));
    }
}
