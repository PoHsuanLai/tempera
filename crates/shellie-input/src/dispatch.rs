//! Turning key presses into command handler calls.
//!
//! Each frame, for every chord that became pressed:
//!
//! 1. find the commands bound to it,
//! 2. drop the ones whose [`When`] does not hold,
//! 3. run the highest-[`Priority`] survivor's [`OnPress`],
//! 4. if it returned a payload and the command has an [`OnRelease`], record a
//!    claim on that chord.
//!
//! and for every chord that became released, hand the recorded payload back to
//! the command that claimed it.
//!
//! # The pairing invariant
//!
//! **Release goes to whoever claimed the press, and its condition is not
//! re-checked.** Conditions describe whether an action should *start*, and a
//! held action's world changes while it is held — that is the whole point of
//! holding it. Re-evaluating on release means a note whose track was switched
//! mid-hold never gets its note-off, and hangs forever.
//!
//! This is why the claim stores the entity and the payload rather than
//! re-deriving them, and why [`HeldClaims`] is keyed by the chord's keys: the
//! release we must match is "the same physical keys came up", regardless of
//! what else moved.
//!
//! # Losing the window
//!
//! A held command whose key is still down when the window loses focus would
//! otherwise never see its release — alt-tab with a note held and it sustains
//! forever. [`drain_held_claims`] releases every outstanding claim on focus
//! loss and on exit, so this is handled once here rather than by every held
//! command individually.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::chord::Chord;
use crate::command::{CommandId, HeldPayload, Keybind, OnPress, OnRelease};
use crate::condition::{Priority, When};

/// The keys of a chord, normalized for use as a claim key.
///
/// Sorted so that a chord's identity does not depend on the order its keys
/// were spelled in.
#[derive(Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ChordKeys(Vec<KeyCode>);

impl ChordKeys {
    fn of(chord: &Chord) -> Self {
        let mut keys = chord.keys();
        keys.sort();
        Self(keys)
    }

    /// Every key of the chord is currently held.
    fn all_held(&self, keys: &ButtonInput<KeyCode>) -> bool {
        !self.0.is_empty() && self.0.iter().all(|k| keys.pressed(*k))
    }

    /// The chord just became fully held — every key down, and the last one
    /// arrived this frame.
    ///
    /// Requiring one `just_pressed` is what stops a chord from re-firing every
    /// frame while held, without needing any per-chord edge state.
    fn just_completed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        self.all_held(keys) && self.0.iter().any(|k| keys.just_pressed(*k))
    }
}

/// Presses that are waiting for their release.
///
/// One entry per chord: `chord -> (the command that claimed it, its payload)`.
#[derive(Resource, Default)]
pub struct HeldClaims {
    claims: Vec<(ChordKeys, Entity, HeldPayload)>,
}

impl HeldClaims {
    fn insert(&mut self, chord: ChordKeys, entity: Entity, payload: HeldPayload) {
        self.claims.retain(|(c, _, _)| *c != chord);
        self.claims.push((chord, entity, payload));
    }

    fn take(&mut self, chord: &ChordKeys) -> Option<(Entity, HeldPayload)> {
        let idx = self.claims.iter().position(|(c, _, _)| c == chord)?;
        let (_, entity, payload) = self.claims.remove(idx);
        Some((entity, payload))
    }

    fn take_all(&mut self) -> Vec<(ChordKeys, Entity, HeldPayload)> {
        std::mem::take(&mut self.claims)
    }

    pub fn len(&self) -> usize {
        self.claims.len()
    }

    pub fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }
}

/// A command that might fire, gathered before any handler runs.
struct Candidate {
    entity: Entity,
    chord: ChordKeys,
    priority: i32,
    has_release: bool,
}

/// Run pressed commands and settle released ones.
///
/// Exclusive because handlers take `&mut World`: they are arbitrary
/// application code that writes messages, mutates resources, and spawns.
pub fn dispatch_commands(world: &mut World) {
    settle_releases(world);
    run_presses(world);
}

/// Hand back payloads for chords that are no longer fully held.
fn settle_releases(world: &mut World) {
    let released: Vec<ChordKeys> = {
        let Some(keys) = world.get_resource::<ButtonInput<KeyCode>>() else {
            return;
        };
        let Some(held) = world.get_resource::<HeldClaims>() else {
            return;
        };
        held.claims
            .iter()
            .map(|(chord, _, _)| chord)
            .filter(|chord| !chord.all_held(keys))
            .cloned()
            .collect()
    };

    for chord in released {
        let Some((entity, payload)) = world.resource_mut::<HeldClaims>().take(&chord) else {
            continue;
        };
        // Deliberately not re-checking `When`: see the pairing invariant.
        let Some(release) = world.get::<OnRelease>(entity).map(|r| r.0.clone()) else {
            continue;
        };
        release(world, payload);
    }
}

/// Fire the winning command for each chord completed this frame.
fn run_presses(world: &mut World) {
    let candidates = collect_candidates(world);
    if candidates.is_empty() {
        return;
    }

    for candidate in pick_winners(world, candidates) {
        let Some(press) = world.get::<OnPress>(candidate.entity).map(|p| p.0.clone()) else {
            continue;
        };
        let payload = press(world);

        match (payload, candidate.has_release) {
            // A held command that accepted the press: remember it so the
            // release can find it.
            (Some(payload), true) => {
                world.resource_mut::<HeldClaims>().insert(
                    candidate.chord,
                    candidate.entity,
                    payload,
                );
            }
            // Declined (`None`) — no claim, so no release will fire. Or a
            // payload with nowhere to go, which is a registration mistake
            // worth surfacing.
            (Some(_), false) => {
                let id = world
                    .get::<CommandId>(candidate.entity)
                    .map(|c| c.0.clone())
                    .unwrap_or_default();
                warn!(
                    "[shellie-input] command {id:?} returned a held payload but has \
                     no on_release; the payload is dropped"
                );
            }
            (None, _) => {}
        }
    }
}

fn collect_candidates(world: &mut World) -> Vec<Candidate> {
    let Some(keys) = world.get_resource::<ButtonInput<KeyCode>>() else {
        return Vec::new();
    };
    // Cloned so the world is free for condition evaluation below.
    let keys = keys.clone();

    let mut q = world.query::<(Entity, &Keybind, Option<&Priority>, Option<&OnRelease>)>();
    q.iter(world)
        .filter_map(|(entity, bind, priority, release)| {
            let chord = ChordKeys::of(&bind.0);
            chord.just_completed(&keys).then(|| Candidate {
                entity,
                chord,
                priority: priority.copied().unwrap_or_default().0,
                has_release: release.is_some(),
            })
        })
        .collect()
}

/// Keep, per chord, the highest-priority candidate whose condition holds.
fn pick_winners(world: &mut World, candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut passing: Vec<Candidate> = Vec::new();

    for candidate in candidates {
        if !condition_holds(world, candidate.entity) {
            continue;
        }
        match passing.iter_mut().find(|c| c.chord == candidate.chord) {
            Some(existing) if existing.priority < candidate.priority => *existing = candidate,
            Some(_) => {}
            None => passing.push(candidate),
        }
    }
    passing
}

/// Evaluate a command's `When`, if it has one. No condition means yes.
///
/// The component is temporarily removed so the condition can be run against
/// `&World` — evaluation needs `&mut When` (conditions may hold `Local` state)
/// and a shared world at the same time, which the borrow checker will not give
/// us while the component is in place.
fn condition_holds(world: &mut World, entity: Entity) -> bool {
    let Some(mut when) = world.entity_mut(entity).take::<When>() else {
        return true;
    };
    when.initialize(world);
    let passed = when.eval(world);
    world.entity_mut(entity).insert(when);
    passed
}

/// Release every outstanding claim.
///
/// Runs when the window loses focus or the app exits, so a held command cannot
/// be stranded by an alt-tab.
pub fn drain_held_claims(world: &mut World) {
    let claims = world.resource_mut::<HeldClaims>().take_all();
    for (_, entity, payload) in claims {
        if let Some(release) = world.get::<OnRelease>(entity).map(|r| r.0.clone()) {
            release(world, payload);
        }
    }
}

/// True when a window exists and none of them has focus.
///
/// The "a window exists" half matters: `all()` on an empty iterator is `true`,
/// so a headless app — a test, a CI run, an offline render — would otherwise
/// look permanently unfocused and drain every held claim the frame after it
/// was made.
pub(crate) fn window_lost_focus(windows: Query<&Window, With<PrimaryWindow>>) -> bool {
    let mut windows = windows.iter().peekable();
    windows.peek().is_some() && windows.all(|w| !w.focused)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord::{cmd, key};
    use crate::command::{
        AppCommandExt, Command, CommandLabel, on_press, on_press_held, on_release,
    };
    use crate::plugin::ShellieInputPlugin;

    #[derive(Resource, Default, Debug, PartialEq)]
    struct Log(Vec<String>);

    fn log(world: &mut World, entry: impl Into<String>) {
        world.resource_mut::<Log>().0.push(entry.into());
    }

    struct Split;
    impl Command for Split {
        const ID: &'static str = "test.split";
    }
    struct DeleteTrack;
    impl Command for DeleteTrack {
        const ID: &'static str = "test.delete_track";
    }
    struct PlayNote;
    impl Command for PlayNote {
        const ID: &'static str = "test.play_note";
    }

    #[derive(Resource, Default)]
    struct Gate(bool);

    fn gate_open(g: Res<Gate>) -> bool {
        g.0
    }

    fn test_app() -> App {
        let mut app = App::new();
        // The plugin already registers `dispatch_commands`; adding it here too
        // would run every handler twice.
        app.add_plugins(ShellieInputPlugin::new("shellie-test-unused"));
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<Log>();
        app.init_resource::<Gate>();
        app
    }

    fn press(app: &mut App, k: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(k);
    }

    fn release(app: &mut App, k: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(k);
    }

    /// Advance one frame, then clear `just_pressed` the way Bevy's input
    /// plugin would.
    fn frame(app: &mut App) {
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }

    #[test]
    fn a_bound_chord_runs_its_handler() {
        let mut app = test_app();
        app.spawn_command(cmd_bundle::<Split>("split", key(KeyCode::KeyS)));

        press(&mut app, KeyCode::KeyS);
        frame(&mut app);

        assert_eq!(app.world().resource::<Log>().0, vec!["split"]);
    }

    fn cmd_bundle<C: Command>(name: &'static str, chord: Chord) -> impl Bundle {
        crate::command::cmd::<C>((
            CommandLabel::new(name),
            Keybind(chord),
            on_press(move |w: &mut World| log(w, name)),
        ))
    }

    #[test]
    fn a_held_chord_does_not_refire_every_frame() {
        let mut app = test_app();
        app.spawn_command(cmd_bundle::<Split>("split", key(KeyCode::KeyS)));

        press(&mut app, KeyCode::KeyS);
        frame(&mut app);
        frame(&mut app); // still held, but no new just_pressed
        frame(&mut app);

        assert_eq!(app.world().resource::<Log>().0, vec!["split"]);
    }

    #[test]
    fn a_failing_condition_blocks_the_command() {
        let mut app = test_app();
        app.spawn_command(crate::command::cmd::<Split>((
            CommandLabel::new("split"),
            Keybind(key(KeyCode::KeyS)),
            When::new(gate_open),
            on_press(|w: &mut World| log(w, "split")),
        )));

        press(&mut app, KeyCode::KeyS);
        frame(&mut app);
        assert!(app.world().resource::<Log>().0.is_empty(), "gate shut");

        app.world_mut().resource_mut::<Gate>().0 = true;
        release(&mut app, KeyCode::KeyS);
        frame(&mut app);
        press(&mut app, KeyCode::KeyS);
        frame(&mut app);
        assert_eq!(app.world().resource::<Log>().0, vec!["split"]);
    }

    #[test]
    fn priority_decides_between_two_applicable_commands() {
        // The Delete collision: piano-roll delete-note vs global delete-track.
        let mut app = test_app();
        app.spawn_command(crate::command::cmd::<Split>((
            CommandLabel::new("note"),
            Keybind(key(KeyCode::Delete)),
            Priority(10),
            on_press(|w: &mut World| log(w, "delete-note")),
        )));
        app.spawn_command(crate::command::cmd::<DeleteTrack>((
            CommandLabel::new("track"),
            Keybind(key(KeyCode::Delete)),
            on_press(|w: &mut World| log(w, "delete-track")),
        )));

        press(&mut app, KeyCode::Delete);
        frame(&mut app);

        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["delete-note"],
            "only the winner runs"
        );
    }

    #[test]
    fn a_lower_priority_command_wins_when_the_higher_one_is_gated_out() {
        let mut app = test_app();
        app.spawn_command(crate::command::cmd::<Split>((
            CommandLabel::new("note"),
            Keybind(key(KeyCode::Delete)),
            Priority(10),
            When::new(gate_open),
            on_press(|w: &mut World| log(w, "delete-note")),
        )));
        app.spawn_command(crate::command::cmd::<DeleteTrack>((
            CommandLabel::new("track"),
            Keybind(key(KeyCode::Delete)),
            on_press(|w: &mut World| log(w, "delete-track")),
        )));

        press(&mut app, KeyCode::Delete);
        frame(&mut app);

        assert_eq!(app.world().resource::<Log>().0, vec!["delete-track"]);
    }

    #[test]
    fn a_chord_needs_every_key() {
        let mut app = test_app();
        app.spawn_command(cmd_bundle::<Split>("save", cmd(KeyCode::KeyS)));

        press(&mut app, KeyCode::KeyS); // no modifier
        frame(&mut app);
        assert!(app.world().resource::<Log>().0.is_empty());
    }

    // ── held commands ────────────────────────────────────────────────────

    fn spawn_note_command(app: &mut App) {
        app.spawn_command(crate::command::cmd::<PlayNote>((
            CommandLabel::new("note"),
            Keybind(key(KeyCode::KeyA)),
            on_press_held(|w: &mut World| {
                log(w, "note-on");
                Some(60u8)
            }),
            on_release(|w: &mut World, pitch: u8| log(w, format!("note-off:{pitch}"))),
        )));
    }

    #[test]
    fn press_and_release_pair_up() {
        let mut app = test_app();
        spawn_note_command(&mut app);

        press(&mut app, KeyCode::KeyA);
        frame(&mut app);
        assert_eq!(app.world().resource::<Log>().0, vec!["note-on"]);

        release(&mut app, KeyCode::KeyA);
        frame(&mut app);
        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["note-on", "note-off:60"]
        );
    }

    #[test]
    fn release_ignores_a_condition_that_flipped_mid_hold() {
        // The hung-note case. The condition was true at press; it goes false
        // while the key is held. Release must still fire, or the note sustains
        // forever.
        let mut app = test_app();
        app.world_mut().resource_mut::<Gate>().0 = true;
        app.spawn_command(crate::command::cmd::<PlayNote>((
            CommandLabel::new("note"),
            Keybind(key(KeyCode::KeyA)),
            When::new(gate_open),
            on_press_held(|w: &mut World| {
                log(w, "note-on");
                Some(60u8)
            }),
            on_release(|w: &mut World, pitch: u8| log(w, format!("note-off:{pitch}"))),
        )));

        press(&mut app, KeyCode::KeyA);
        frame(&mut app);

        app.world_mut().resource_mut::<Gate>().0 = false;
        release(&mut app, KeyCode::KeyA);
        frame(&mut app);

        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["note-on", "note-off:60"],
            "release must not be gated"
        );
    }

    #[test]
    fn a_declined_press_records_no_claim() {
        let mut app = test_app();
        app.spawn_command(crate::command::cmd::<PlayNote>((
            CommandLabel::new("note"),
            Keybind(key(KeyCode::KeyA)),
            // No target track, say — nothing to play, so decline.
            on_press_held(|w: &mut World| {
                log(w, "declined");
                None::<u8>
            }),
            on_release(|w: &mut World, _: u8| log(w, "note-off")),
        )));

        press(&mut app, KeyCode::KeyA);
        frame(&mut app);
        release(&mut app, KeyCode::KeyA);
        frame(&mut app);

        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["declined"],
            "a declined press must not produce a release"
        );
        assert!(app.world().resource::<HeldClaims>().is_empty());
    }

    #[test]
    fn releasing_any_key_of_a_chord_settles_the_claim() {
        let mut app = test_app();
        app.spawn_command(crate::command::cmd::<PlayNote>((
            CommandLabel::new("note"),
            Keybind(cmd(KeyCode::KeyA)),
            on_press_held(|w: &mut World| {
                log(w, "on");
                Some(1u8)
            }),
            on_release(|w: &mut World, _: u8| log(w, "off")),
        )));

        let modifier = if cfg!(target_os = "macos") {
            KeyCode::SuperLeft
        } else {
            KeyCode::ControlLeft
        };
        press(&mut app, modifier);
        press(&mut app, KeyCode::KeyA);
        frame(&mut app);
        assert_eq!(app.world().resource::<Log>().0, vec!["on"]);

        // Let go of the modifier only: the chord is no longer held.
        release(&mut app, modifier);
        frame(&mut app);
        assert_eq!(app.world().resource::<Log>().0, vec!["on", "off"]);
    }

    #[test]
    fn draining_releases_everything_outstanding() {
        let mut app = test_app();
        spawn_note_command(&mut app);

        press(&mut app, KeyCode::KeyA);
        frame(&mut app);
        assert_eq!(app.world().resource::<HeldClaims>().len(), 1);

        // Window focus lost with the key still down.
        drain_held_claims(app.world_mut());

        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["note-on", "note-off:60"],
            "an alt-tab must not strand a held command"
        );
        assert!(app.world().resource::<HeldClaims>().is_empty());
    }

    #[test]
    fn a_headless_app_does_not_look_permanently_unfocused() {
        // `all()` over zero windows is `true`, so a naive focus check drains
        // every claim the frame after it is made — note-on, note-off, with the
        // key still down.
        let mut app = test_app();
        spawn_note_command(&mut app);

        press(&mut app, KeyCode::KeyA);
        frame(&mut app);
        frame(&mut app);

        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["note-on"],
            "no window at all is not the same as an unfocused window"
        );
        assert_eq!(app.world().resource::<HeldClaims>().len(), 1);
    }

    #[test]
    fn a_drained_claim_does_not_fire_again_on_the_real_release() {
        let mut app = test_app();
        spawn_note_command(&mut app);

        press(&mut app, KeyCode::KeyA);
        frame(&mut app);
        drain_held_claims(app.world_mut());

        release(&mut app, KeyCode::KeyA);
        frame(&mut app);

        assert_eq!(
            app.world().resource::<Log>().0,
            vec!["note-on", "note-off:60"],
            "exactly one release"
        );
    }
}
