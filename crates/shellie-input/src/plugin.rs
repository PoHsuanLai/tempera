//! The plugin, and the one piece of configuration shellie needs from its host.

use bevy::prelude::*;

use crate::binding::{apply_saved_bindings, strip_unbound_keybinds};
use crate::command::{CommandRegistry, unregister_despawned_commands};
use crate::dispatch::{HeldClaims, dispatch_commands, drain_held_claims, window_lost_focus};
use crate::persist::SavedKeybinds;

/// The application's name, used to namespace on-disk state.
///
/// shellie does not know what application it is running. Two apps built on it
/// must not share one keybinds file, so the name is supplied rather than
/// assumed.
#[derive(Resource, Clone, Debug)]
pub struct AppName(pub String);

/// Keybinds and commands.
///
/// ```
/// use bevy::prelude::*;
/// use shellie_input::plugin::ShellieInputPlugin;
///
/// let mut app = App::new();
/// app.add_plugins(ShellieInputPlugin::new("my-app"));
/// ```
pub struct ShellieInputPlugin {
    app_name: String,
}

impl ShellieInputPlugin {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
        }
    }
}

impl Plugin for ShellieInputPlugin {
    fn build(&self, app: &mut App) {
        let saved = SavedKeybinds::load(&self.app_name);

        app.insert_resource(AppName(self.app_name.clone()))
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
            .add_systems(Update, dispatch_commands.in_set(CommandDispatch))
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

    struct Undo;
    impl Command for Undo {
        const ID: &'static str = "edit.undo";
    }

    #[test]
    fn registry_upkeep_is_wired_by_the_plugin() {
        let mut app = App::new();
        app.add_plugins(ShellieInputPlugin::new("shellie-test-unused"));
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
        app.add_plugins(ShellieInputPlugin::new("shellie-test-unused"));
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
