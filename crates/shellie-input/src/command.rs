//! Commands: what a keypress can do.
//!
//! A command is an **entity**. That is not an implementation detail — it is
//! the extension model. The components below are the ones shellie knows how
//! to interpret; an application is free to add its own to the same entity and
//! query them, which is how things shellie has no business modelling (a
//! palette grouping, an icon, an owning extension id) get attached without
//! shellie growing a field for them.
//!
//! ```
//! use bevy::prelude::*;
//! use shellie_input::chord::cmd as chord;
//! use shellie_input::command::{AppCommandExt, Command, CommandLabel, Keybind, cmd, on_press};
//! use shellie_input::plugin::ShellieInputPlugin;
//!
//! struct Undo;
//! impl Command for Undo {
//!     const ID: &'static str = "edit.undo";
//! }
//!
//! # #[derive(Resource, Default)] struct Undos(usize);
//! let mut app = App::new();
//! # app.init_resource::<Undos>();
//! app.add_plugins(ShellieInputPlugin::new("my-app"));
//! app.spawn_command(cmd::<Undo>((
//!     CommandLabel::new("Undo"),
//!     Keybind(chord(KeyCode::KeyZ)),
//!     on_press(|w: &mut World| { w.resource_mut::<Undos>().0 += 1; }),
//! )));
//!
//! // Firing by type is checked at compile time.
//! shellie_input::fire::<Undo>(app.world_mut());
//! # assert_eq!(app.world().resource::<Undos>().0, 1);
//! ```
//!
//! Optional behaviour is an **absent component**, never an `Option` field: a
//! command with no [`When`] is unconditional, one with no [`Priority`] sits at
//! zero, one with no [`OnRelease`] ignores key release. This is why the
//! registration call did not need to grow a seventh positional argument to
//! gain conditions.
//!
//! # Identity
//!
//! [`Command::ID`] is an associated const so [`fire`] can be checked at
//! compile time — `fire::<Undo>(world)` cannot typo the way
//! `fire_by_id(world, "edit.undo")` can. That is the trait's entire job.
//!
//! Ids remain strings on the wire and on disk, deliberately: extension
//! manifests and recent-file lists mint them at runtime, and the keybind file
//! is keyed by them. [`dyn_cmd`] serves those callers and
//! produces the very same components, so nothing downstream can tell the two
//! registration paths apart.

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::chord::Chord;

// ── identity ─────────────────────────────────────────────────────────────

/// A statically-known command.
///
/// Implement this for a unit struct per command; the type is the compile-time
/// handle used by [`fire`].
pub trait Command: 'static {
    /// Stable identity. Also the persistence key for keybind overrides, so
    /// changing it discards users' customizations for this command.
    const ID: &'static str;
}

/// Stable string identity, as stored on the entity.
///
/// A `String` rather than `&'static str` because [`dyn_cmd`]
/// mints ids at runtime from extension manifests.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub String);

impl CommandId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Human-readable name, for the palette and the keybind settings list.
///
/// Owned rather than `&'static str` so it can be localized or built at
/// runtime.
#[derive(Component, Clone, Debug)]
pub struct CommandLabel(pub Cow<'static, str>);

impl CommandLabel {
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self(label.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Marker for every command entity, so the palette can query them without
/// naming any other component.
#[derive(Component, Default)]
pub struct CommandMarker;

// ── binding ──────────────────────────────────────────────────────────────

/// The chord that triggers this command.
///
/// A command without one is still perfectly usable — it just has to be fired
/// by id, which is what palette-only entries do.
#[derive(Component, Clone, Debug)]
pub struct Keybind(pub Chord);

/// The scope this command's binding belongs to, for persistence grouping.
///
/// Scopes exist *only* so the keybind file and the settings UI can group
/// related bindings. They do not gate dispatch — that is [`When`]'s job.
#[derive(Component, Clone, Debug)]
pub struct BindScope(pub Cow<'static, str>);

impl Default for BindScope {
    fn default() -> Self {
        Self(Cow::Borrowed("global"))
    }
}

// ── handlers ─────────────────────────────────────────────────────────────

/// What runs when the chord is pressed.
///
/// Returns an opaque payload that, if the command also has an [`OnRelease`],
/// is handed back on release. Returning `None` means "declined": no claim is
/// recorded, and release will not fire. That is not an error path — it is how
/// a command says "the preconditions I could only check while acting were not
/// met", which is exactly the case a [`When`] condition cannot express.
#[derive(Component, Clone)]
pub struct OnPress(pub Arc<dyn Fn(&mut World) -> Option<HeldPayload> + Send + Sync>);

/// What runs when a held command's chord is released.
///
/// Receives the payload its own [`OnPress`] produced. Absent for ordinary
/// fire-and-forget commands.
#[derive(Component, Clone)]
pub struct OnRelease(pub Arc<dyn Fn(&mut World, HeldPayload) + Send + Sync>);

/// Opaque payload handed from a press to its matching release.
///
/// The concrete type is the application's and is never named by shellie. The
/// downcast in [`on_release`] is unreachable-by-construction: the only thing
/// that boxes a value of type `T` is an [`on_press`] closure, and the only
/// thing that unboxes it is the [`on_release`] closure monomorphized beside
/// it at the same call site.
pub type HeldPayload = Box<dyn Any + Send + Sync>;

/// Build an [`OnPress`] for a fire-and-forget command.
pub fn on_press(f: impl Fn(&mut World) + Send + Sync + 'static) -> OnPress {
    OnPress(Arc::new(move |world| {
        f(world);
        None
    }))
}

/// Build an [`OnPress`] for a held command.
///
/// Returning `None` declines the press: nothing is recorded and the paired
/// [`on_release`] will not run.
pub fn on_press_held<T: Send + Sync + 'static>(
    f: impl Fn(&mut World) -> Option<T> + Send + Sync + 'static,
) -> OnPress {
    OnPress(Arc::new(move |world| {
        f(world).map(|v| Box::new(v) as HeldPayload)
    }))
}

/// Build an [`OnRelease`] that receives the payload from [`on_press_held`].
///
/// A payload of the wrong type is dropped with a warning rather than
/// panicking. It cannot happen through the public API — the pair is
/// monomorphized together — so it would mean shellie itself mismatched a
/// claim, and killing the app over it helps nobody.
pub fn on_release<T: Send + Sync + 'static>(
    f: impl Fn(&mut World, T) + Send + Sync + 'static,
) -> OnRelease {
    OnRelease(Arc::new(move |world, payload| {
        match payload.downcast::<T>() {
            Ok(v) => f(world, *v),
            Err(_) => {
                tracing::error!(
                    "[shellie-input] held payload type mismatch on release; \
                 the press/release pair disagreed about its payload type"
                );
            }
        }
    }))
}

// ── registry ─────────────────────────────────────────────────────────────

/// `command id -> entity`, maintained by shellie.
///
/// Exists for two reasons. It makes firing by id a hash lookup rather than a
/// scan over every command entity; and it makes duplicate ids *visible*.
/// Without it, two commands registering the same id both exist and dispatch
/// silently picks whichever the query happens to yield first — which for
/// extension-supplied ids means an extension can shadow a built-in and nobody
/// finds out.
#[derive(Resource, Default, Debug)]
pub struct CommandRegistry {
    by_id: HashMap<String, Entity>,
}

impl CommandRegistry {
    pub fn get(&self, id: &str) -> Option<Entity> {
        self.by_id.get(id).copied()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Entity)> {
        self.by_id.iter().map(|(id, e)| (id.as_str(), *e))
    }

    /// Register `id -> entity`, rejecting a duplicate.
    ///
    /// The first registration wins: a later collision is refused and logged,
    /// so a built-in cannot be displaced by an extension that reuses its id.
    fn insert(&mut self, id: String, entity: Entity) -> bool {
        if let Some(existing) = self.by_id.get(&id) {
            error!(
                "[shellie-input] duplicate command id {id:?}: already registered \
                 as {existing:?}, refusing {entity:?}"
            );
            return false;
        }
        self.by_id.insert(id, entity);
        true
    }

    /// Drop `id`, but only if `entity` is the one currently registered under
    /// it.
    ///
    /// The entity check is load-bearing. A rejected duplicate is despawned
    /// while still carrying the id it failed to claim; without this guard its
    /// removal would evict the *original's* entry and leave the id
    /// unresolvable.
    fn remove_if_owner(&mut self, id: &str, entity: Entity) {
        if self.by_id.get(id) == Some(&entity) {
            self.by_id.remove(id);
        }
    }
}

/// Keep [`CommandRegistry`] in step with despawned command entities, so a
/// removed command's id becomes available again.
pub(crate) fn unregister_despawned_commands(
    remove: On<Remove, CommandId>,
    ids: Query<&CommandId>,
    mut registry: ResMut<CommandRegistry>,
) {
    if let Ok(id) = ids.get(remove.entity) {
        registry.remove_if_owner(id.as_str(), remove.entity);
    }
}

// ── spawning ─────────────────────────────────────────────────────────────

/// Attach a statically-known [`Command`]'s identity to a bundle.
///
/// ```
/// use bevy::prelude::*;
/// use shellie_input::chord::key;
/// use shellie_input::command::{AppCommandExt, Command, CommandLabel, Keybind, cmd, on_press};
/// use shellie_input::plugin::ShellieInputPlugin;
///
/// struct Save;
/// impl Command for Save {
///     const ID: &'static str = "file.save";
/// }
///
/// let mut app = App::new();
/// app.add_plugins(ShellieInputPlugin::new("my-app"));
/// app.spawn_command(cmd::<Save>((
///     CommandLabel::new("Save"),
///     Keybind(key(KeyCode::F5)),
///     on_press(|_: &mut World| {}),
/// )));
/// ```
pub fn cmd<C: Command>(extra: impl Bundle) -> impl Bundle {
    (CommandMarker, CommandId(C::ID.to_owned()), extra)
}

/// Attach a runtime id to a bundle, for commands that do not exist at compile
/// time — extension manifests, recent-file entries.
///
/// Produces the same components as [`cmd`]; only the compile-time [`fire`]
/// checking is forfeited.
pub fn dyn_cmd(id: impl Into<String>, extra: impl Bundle) -> impl Bundle {
    (CommandMarker, CommandId(id.into()), extra)
}

/// Registration entry points.
pub trait AppCommandExt {
    /// Spawn a command entity and index it.
    ///
    /// The bundle must carry a [`CommandId`] — use [`cmd`] or [`dyn_cmd`] to
    /// attach one.
    fn spawn_command(&mut self, bundle: impl Bundle) -> &mut Self;
}

impl AppCommandExt for App {
    fn spawn_command(&mut self, bundle: impl Bundle) -> &mut Self {
        let world = self.world_mut();
        world.init_resource::<CommandRegistry>();

        let entity = world.spawn(bundle).id();

        let Some(id) = world.get::<CommandId>(entity).map(|c| c.0.clone()) else {
            error!(
                "[shellie-input] spawn_command called with a bundle carrying no \
                 CommandId; despawning. Wrap it in cmd::<T>() or dyn_cmd(id, ..)."
            );
            world.entity_mut(entity).despawn();
            return self;
        };

        world
            .entity_mut(entity)
            .insert(Name::new(format!("cmd:{id}")));

        if !world.resource_mut::<CommandRegistry>().insert(id, entity) {
            // Duplicate id — the first registration keeps the name.
            world.entity_mut(entity).despawn();
        }
        self
    }
}

// ── firing ───────────────────────────────────────────────────────────────

/// Run a statically-known command's press handler, ignoring its [`When`].
///
/// This is the *deliberate* path for UI affordances — a toolbar button or a
/// menu item that is only visible when it applies, and whose visibility the
/// caller has already decided. Keyboard dispatch does not use this; it goes
/// through the condition check.
///
/// Returns whether a handler ran.
pub fn fire<C: Command>(world: &mut World) -> bool {
    fire_by_id(world, C::ID)
}

/// [`fire`] for an id only known at runtime.
pub fn fire_by_id(world: &mut World, id: &str) -> bool {
    let Some(entity) = world
        .get_resource::<CommandRegistry>()
        .and_then(|r| r.get(id))
    else {
        warn!("[shellie-input] fire: no command registered as {id:?}");
        return false;
    };
    let Some(press) = world.get::<OnPress>(entity).map(|p| p.0.clone()) else {
        return false;
    };
    // A payload returned here is dropped: firing by id is the fire-and-forget
    // path, and there is no key release coming to pair with it.
    press(world);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Undo;
    impl Command for Undo {
        const ID: &'static str = "edit.undo";
    }

    struct Redo;
    impl Command for Redo {
        const ID: &'static str = "edit.redo";
    }

    #[derive(Resource, Default)]
    struct Log(Vec<&'static str>);

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<CommandRegistry>();
        app.init_resource::<Log>();
        app.add_observer(unregister_despawned_commands);
        app
    }

    #[test]
    fn spawn_indexes_by_id() {
        let mut app = app();
        app.spawn_command(cmd::<Undo>((
            CommandLabel::new("Undo"),
            on_press(|w: &mut World| w.resource_mut::<Log>().0.push("undo")),
        )));

        let registry = app.world().resource::<CommandRegistry>();
        assert_eq!(registry.len(), 1);
        assert!(registry.get("edit.undo").is_some());
    }

    #[test]
    fn fire_is_checked_by_type_and_runs_the_handler() {
        let mut app = app();
        app.spawn_command(cmd::<Undo>((
            CommandLabel::new("Undo"),
            on_press(|w: &mut World| w.resource_mut::<Log>().0.push("undo")),
        )));

        assert!(fire::<Undo>(app.world_mut()));
        assert_eq!(app.world().resource::<Log>().0, vec!["undo"]);
    }

    #[test]
    fn firing_an_unregistered_command_is_a_no_op() {
        let mut app = app();
        assert!(!fire::<Redo>(app.world_mut()));
    }

    #[test]
    fn a_duplicate_id_is_refused_and_the_original_survives() {
        let mut app = app();
        app.spawn_command(cmd::<Undo>((
            CommandLabel::new("first"),
            on_press(|w: &mut World| w.resource_mut::<Log>().0.push("first")),
        )));
        let first = app.world().resource::<CommandRegistry>().get("edit.undo");

        // An extension re-using a built-in id must not displace it.
        app.spawn_command(cmd::<Undo>((
            CommandLabel::new("impostor"),
            on_press(|w: &mut World| w.resource_mut::<Log>().0.push("impostor")),
        )));

        let registry = app.world().resource::<CommandRegistry>();
        assert_eq!(registry.len(), 1, "the duplicate must not be indexed");
        assert_eq!(
            registry.get("edit.undo"),
            first,
            "the original must survive"
        );

        fire::<Undo>(app.world_mut());
        assert_eq!(app.world().resource::<Log>().0, vec!["first"]);
    }

    #[test]
    fn despawning_frees_the_id() {
        let mut app = app();
        app.spawn_command(cmd::<Undo>((CommandLabel::new("Undo"), on_press(|_| {}))));
        let entity = app
            .world()
            .resource::<CommandRegistry>()
            .get("edit.undo")
            .unwrap();

        app.world_mut().entity_mut(entity).despawn();
        assert!(
            app.world()
                .resource::<CommandRegistry>()
                .get("edit.undo")
                .is_none(),
            "a despawned command must release its id"
        );
    }

    #[test]
    fn dynamic_ids_produce_the_same_components() {
        let mut app = app();
        app.spawn_command(dyn_cmd(
            "ext.foo.bar",
            (
                CommandLabel::new("From an extension"),
                on_press(|w: &mut World| w.resource_mut::<Log>().0.push("ext")),
            ),
        ));

        assert!(fire_by_id(app.world_mut(), "ext.foo.bar"));
        assert_eq!(app.world().resource::<Log>().0, vec!["ext"]);
    }

    #[test]
    fn a_bundle_without_an_id_is_rejected_not_leaked() {
        let mut app = app();
        app.spawn_command((CommandLabel::new("orphan"), on_press(|_| {})));

        assert!(app.world().resource::<CommandRegistry>().is_empty());
        let orphans = app
            .world_mut()
            .query::<&CommandLabel>()
            .iter(app.world())
            .count();
        assert_eq!(orphans, 0, "the rejected entity must not be left behind");
    }
}
