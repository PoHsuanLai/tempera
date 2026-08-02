//! Resolving a command's *effective* chord: its registered default, or the
//! user's override if there is one.
//!
//! # The display bug this exists to avoid
//!
//! The implementation this replaces applied overrides to the live input map
//! but *removed* the entry from its display caches, because it had no way to
//! turn a stored key list back into a `Chord`. The visible result: rebind a
//! key and its shortcut hint goes blank everywhere — tooltips, the palette,
//! the settings row you just used to rebind it.
//!
//! [`Chord::deserialize`](crate::chord::Chord::deserialize) removes the cause,
//! so a resolved binding is a `Chord` whether it came from a default or from
//! disk, and every consumer can render it the same way.

use bevy::prelude::*;

use crate::chord::Chord;
use crate::command::{BindScope, CommandId, Keybind};
use crate::persist::SavedKeybinds;

/// A command's effective binding after user overrides are applied.
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    /// The chord registered in code — the user has not touched it.
    Default(Chord),
    /// The user rebound it.
    Custom(Chord),
    /// The user explicitly removed the binding. Distinct from "no binding was
    /// ever registered": this one must not fall back to the default.
    Unbound,
}

impl Binding {
    /// The chord to match and to display, if any.
    pub fn chord(&self) -> Option<&Chord> {
        match self {
            Binding::Default(c) | Binding::Custom(c) => Some(c),
            Binding::Unbound => None,
        }
    }

    pub fn is_customized(&self) -> bool {
        matches!(self, Binding::Custom(_) | Binding::Unbound)
    }
}

/// Resolve one command's binding.
///
/// `default` is what the command registered; `saved` is the user's file.
pub fn resolve(
    saved: &SavedKeybinds,
    scope: &str,
    command_id: &str,
    default: Option<&Chord>,
) -> Option<Binding> {
    match saved.scope(scope).and_then(|s| s.get(command_id)) {
        // Explicitly unbound.
        Some(chords) if chords.is_empty() => Some(Binding::Unbound),
        // Rebound. Only the first chord is used: a command has one binding
        // here, and storing a list keeps the format open for multi-bind later
        // without a migration.
        Some(chords) => match Chord::deserialize(&chords[0]) {
            Some(chord) => Some(Binding::Custom(chord)),
            // The stored chord had no recognizable keys. Fall back rather than
            // silently leaving the command unreachable.
            None => {
                warn!(
                    "[shellie-input] unusable saved binding for {command_id:?}; \
                     using the default"
                );
                default.cloned().map(Binding::Default)
            }
        },
        None => default.cloned().map(Binding::Default),
    }
}

/// Apply every saved override to the live command entities.
///
/// Runs once at startup, after commands are registered. Rebinding at runtime
/// goes through [`rebind`] instead.
pub fn apply_saved_bindings(
    saved: Res<SavedKeybinds>,
    mut commands: Query<(&CommandId, Option<&BindScope>, &mut Keybind)>,
) {
    for (id, scope, mut keybind) in &mut commands {
        let scope = scope.map(|s| s.0.as_ref()).unwrap_or("global");
        if let Some(Binding::Custom(chord)) = resolve(&saved, scope, id.as_str(), Some(&keybind.0))
        {
            keybind.0 = chord;
        }
        // `Unbound` is handled by removing the component, which needs deferred
        // `Commands`; see `strip_unbound_keybinds`.
    }
}

/// Remove `Keybind` from commands the user explicitly unbound.
///
/// Separate from [`apply_saved_bindings`] because removing a component needs
/// deferred `Commands` while rewriting one needs `&mut` access.
pub fn strip_unbound_keybinds(
    saved: Res<SavedKeybinds>,
    commands: Query<(Entity, &CommandId, Option<&BindScope>), With<Keybind>>,
    mut cmd: Commands,
) {
    for (entity, id, scope) in &commands {
        let scope = scope.map(|s| s.0.as_ref()).unwrap_or("global");
        if let Some(Binding::Unbound) = resolve(&saved, scope, id.as_str(), None) {
            cmd.entity(entity).remove::<Keybind>();
        }
    }
}

/// Change a command's binding and record it, so it survives a restart.
///
/// Pass `None` to unbind.
pub fn rebind(
    world: &mut World,
    command_id: &str,
    chord: Option<Chord>,
) -> Result<(), RebindError> {
    let entity = world
        .get_resource::<crate::command::CommandRegistry>()
        .and_then(|r| r.get(command_id))
        .ok_or(RebindError::UnknownCommand)?;

    let scope = world
        .get::<BindScope>(entity)
        .map(|s| s.0.to_string())
        .unwrap_or_else(|| "global".to_owned());

    match &chord {
        Some(chord) => {
            world.entity_mut(entity).insert(Keybind(chord.clone()));
            world
                .resource_mut::<SavedKeybinds>()
                .set(&scope, command_id, vec![chord.serialize()]);
        }
        None => {
            world.entity_mut(entity).remove::<Keybind>();
            world
                .resource_mut::<SavedKeybinds>()
                .set(&scope, command_id, vec![]);
        }
    }
    Ok(())
}

/// Restore a command's registered default, discarding the user's override.
pub fn reset_binding(
    world: &mut World,
    command_id: &str,
    default: Chord,
) -> Result<(), RebindError> {
    let entity = world
        .get_resource::<crate::command::CommandRegistry>()
        .and_then(|r| r.get(command_id))
        .ok_or(RebindError::UnknownCommand)?;

    let scope = world
        .get::<BindScope>(entity)
        .map(|s| s.0.to_string())
        .unwrap_or_else(|| "global".to_owned());

    world.entity_mut(entity).insert(Keybind(default));
    world
        .resource_mut::<SavedKeybinds>()
        .clear(&scope, command_id);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebindError {
    UnknownCommand,
}

impl std::fmt::Display for RebindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RebindError::UnknownCommand => write!(f, "no command registered with that id"),
        }
    }
}

impl std::error::Error for RebindError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chord::{cmd, key};

    fn saved_with(scope: &str, id: &str, chords: Vec<Vec<&str>>) -> SavedKeybinds {
        let mut saved = SavedKeybinds::default();
        saved.set(
            scope,
            id,
            chords
                .into_iter()
                .map(|c| c.into_iter().map(String::from).collect())
                .collect(),
        );
        saved
    }

    #[test]
    fn an_untouched_command_keeps_its_default() {
        let saved = SavedKeybinds::default();
        let default = cmd(KeyCode::KeyZ);
        assert_eq!(
            resolve(&saved, "global", "edit.undo", Some(&default)),
            Some(Binding::Default(default))
        );
    }

    #[test]
    fn a_rebound_command_is_still_displayable() {
        // The regression this module exists for: after rebinding, the chord
        // must still be recoverable so shortcut hints keep rendering.
        let saved = saved_with("global", "edit.undo", vec![vec!["Mod", "Y"]]);
        let binding =
            resolve(&saved, "global", "edit.undo", Some(&cmd(KeyCode::KeyZ))).expect("resolved");

        assert!(binding.is_customized());
        let chord = binding.chord().expect("a custom binding still has a chord");
        assert_eq!(chord.keys(), cmd(KeyCode::KeyY).keys());
    }

    #[test]
    fn unbinding_is_not_the_same_as_never_touching() {
        let saved = saved_with("global", "edit.undo", vec![]);
        let binding = resolve(&saved, "global", "edit.undo", Some(&cmd(KeyCode::KeyZ)));

        assert_eq!(binding, Some(Binding::Unbound));
        assert!(
            binding.unwrap().chord().is_none(),
            "an unbound command must not fall back to its default"
        );
    }

    #[test]
    fn an_unusable_saved_binding_falls_back_rather_than_stranding_the_command() {
        let saved = saved_with("global", "edit.undo", vec![vec!["NoSuchKey"]]);
        let default = cmd(KeyCode::KeyZ);
        assert_eq!(
            resolve(&saved, "global", "edit.undo", Some(&default)),
            Some(Binding::Default(default)),
            "a corrupt entry must leave the command reachable"
        );
    }

    #[test]
    fn scopes_do_not_leak_into_each_other() {
        let saved = saved_with("timeline", "edit.undo", vec![vec!["Mod", "Y"]]);
        // Same command id, different scope: the override must not apply.
        assert_eq!(
            resolve(&saved, "global", "edit.undo", Some(&key(KeyCode::KeyZ))),
            Some(Binding::Default(key(KeyCode::KeyZ)))
        );
    }
}
