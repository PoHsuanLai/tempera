//! End-to-end checks through a real `App`: registration, override
//! application, dispatch, and rebinding.
//!
//! The unit tests cover each stage in isolation; these exist to catch the
//! seams between them — an override that resolves correctly but is applied at
//! the wrong point in startup, say, would pass every unit test and still leave
//! the user's custom keybind inert.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use tempera_input::binding::{Binding, resolve};
use tempera_input::chord::{cmd, key};
use tempera_input::command::{
    AppCommandExt, BindScope, Command, CommandLabel, CommandRegistry, Keybind, on_press,
};
use tempera_input::persist::SavedKeybinds;
use tempera_input::plugin::TemperaInputPlugin;
use tempera_input::{Chord, rebind};

#[derive(Resource, Default)]
struct Fired(Vec<&'static str>);

struct Save;
impl Command for Save {
    const ID: &'static str = "file.save";
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(TemperaInputPlugin::new("tempera-e2e-test-unused"));
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<Fired>();
    app
}

fn register_save(app: &mut App) {
    app.spawn_command(tempera_input::cmd::<Save>((
        CommandLabel::new("Save"),
        BindScope("global".into()),
        Keybind(cmd(KeyCode::KeyS)),
        on_press(|w: &mut World| w.resource_mut::<Fired>().0.push("save")),
    )));
}

fn press_chord(app: &mut App, chord: &Chord) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    for k in chord.keys() {
        keys.press(k);
    }
}

fn frame(app: &mut App) {
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

#[test]
fn a_registered_command_fires_on_its_default_chord() {
    let mut app = test_app();
    register_save(&mut app);

    press_chord(&mut app, &cmd(KeyCode::KeyS));
    frame(&mut app);

    assert_eq!(app.world().resource::<Fired>().0, vec!["save"]);
}

#[test]
fn a_saved_override_takes_effect_at_startup() {
    let mut app = test_app();
    // Simulate a config on disk: Save is rebound to Mod+K.
    app.world_mut().resource_mut::<SavedKeybinds>().set(
        "global",
        "file.save",
        vec![vec!["Mod".into(), "K".into()]],
    );
    register_save(&mut app);

    // PostStartup applies overrides; the first update runs it.
    press_chord(&mut app, &cmd(KeyCode::KeyS));
    frame(&mut app);
    assert!(
        app.world().resource::<Fired>().0.is_empty(),
        "the default chord must stop working once rebound"
    );

    press_chord(&mut app, &cmd(KeyCode::KeyK));
    frame(&mut app);
    assert_eq!(app.world().resource::<Fired>().0, vec!["save"]);
}

#[test]
fn a_rebound_command_still_reports_a_displayable_chord() {
    // The regression that motivated this crate's binding module: the original
    // implementation dropped the chord from its display caches on override, so
    // rebinding a key blanked its shortcut hint everywhere.
    let mut app = test_app();
    register_save(&mut app);
    app.update();

    rebind(app.world_mut(), "file.save", Some(cmd(KeyCode::KeyK))).expect("rebind");

    let saved = app.world().resource::<SavedKeybinds>();
    let binding =
        resolve(saved, "global", "file.save", Some(&cmd(KeyCode::KeyS))).expect("resolves");

    assert!(binding.is_customized());
    assert_eq!(
        binding.chord().expect("still displayable").keys(),
        cmd(KeyCode::KeyK).keys(),
        "a rebound command must still render its shortcut"
    );
}

#[test]
fn rebinding_survives_a_restart() {
    let mut app = test_app();
    register_save(&mut app);
    app.update();

    rebind(app.world_mut(), "file.save", Some(key(KeyCode::F5))).expect("rebind");
    let persisted = app.world().resource::<SavedKeybinds>().clone();

    // A fresh app, loading that config.
    let mut restarted = test_app();
    restarted.world_mut().insert_resource(persisted);
    register_save(&mut restarted);

    press_chord(&mut restarted, &key(KeyCode::F5));
    frame(&mut restarted);

    assert_eq!(
        restarted.world().resource::<Fired>().0,
        vec!["save"],
        "the rebind must outlive the process"
    );
}

#[test]
fn unbinding_leaves_the_command_fireable_by_id_only() {
    let mut app = test_app();
    register_save(&mut app);
    app.update();

    rebind(app.world_mut(), "file.save", None).expect("unbind");

    press_chord(&mut app, &cmd(KeyCode::KeyS));
    frame(&mut app);
    assert!(
        app.world().resource::<Fired>().0.is_empty(),
        "an unbound command must not answer its old chord"
    );

    // Still reachable from a palette or a menu.
    assert!(tempera_input::fire::<Save>(app.world_mut()));
    assert_eq!(app.world().resource::<Fired>().0, vec!["save"]);
}

#[test]
fn rebinding_an_unknown_command_is_an_error_not_a_panic() {
    let mut app = test_app();
    assert!(rebind(app.world_mut(), "nope.missing", Some(key(KeyCode::F5))).is_err());
}

#[test]
fn an_explicitly_unbound_command_does_not_fall_back_to_its_default() {
    let mut app = test_app();
    app.world_mut()
        .resource_mut::<SavedKeybinds>()
        .set("global", "file.save", vec![]);
    register_save(&mut app);

    press_chord(&mut app, &cmd(KeyCode::KeyS));
    frame(&mut app);

    assert!(app.world().resource::<Fired>().0.is_empty());
    assert_eq!(
        resolve(
            app.world().resource::<SavedKeybinds>(),
            "global",
            "file.save",
            Some(&cmd(KeyCode::KeyS))
        ),
        Some(Binding::Unbound)
    );
}

#[test]
fn the_registry_indexes_every_registered_command() {
    let mut app = test_app();
    register_save(&mut app);

    let registry = app.world().resource::<CommandRegistry>();
    assert_eq!(registry.len(), 1);
    assert!(registry.get("file.save").is_some());
}

#[test]
fn a_command_does_not_fire_while_a_chord_is_being_recorded() {
    // The seam this file exists for. `capture_chord` and `dispatch_commands`
    // read the same `ButtonInput`, so without the run condition on dispatch a
    // user rebinding Cmd+S would *save* on every attempt to change it — the
    // recorder would work perfectly and the feature would still be unusable.
    //
    // A unit test on either system alone cannot see this: it is a property of
    // how the plugin schedules them together.
    let mut app = test_app();
    register_save(&mut app);

    app.world_mut()
        .init_resource::<tempera_input::ChordCapture>();

    press_chord(&mut app, &cmd(KeyCode::KeyS));
    frame(&mut app);

    assert!(
        app.world().resource::<Fired>().0.is_empty(),
        "the command fired while its own shortcut was being re-recorded"
    );
}

#[test]
fn dispatch_resumes_once_the_recording_ends() {
    // The other half: suppression must be scoped to the recording, or the
    // first rebind would leave every shortcut dead for the rest of the
    // session.
    let mut app = test_app();
    register_save(&mut app);

    app.world_mut()
        .init_resource::<tempera_input::ChordCapture>();
    frame(&mut app);
    app.world_mut()
        .remove_resource::<tempera_input::ChordCapture>();

    press_chord(&mut app, &cmd(KeyCode::KeyS));
    frame(&mut app);

    assert_eq!(
        app.world().resource::<Fired>().0,
        vec!["save"],
        "dispatch stayed suppressed after the recording ended"
    );
}
