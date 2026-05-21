use bevy::prelude::*;

use crate::kbd::KbdChord;

/// Per-item input to [`super::spawn_command`]. Carry only the data
/// the caller cares about — the spawn helper turns each spec into an
/// entity with the right ECS markers.
#[derive(Clone, Debug)]
pub struct CommandItemSpec {
    /// Stable id the caller uses to dispatch on activation. Tempera
    /// emits this in [`CommandActivated`].
    pub id: String,
    /// Display label.
    pub label: String,
    /// Optional keyboard shortcut shown right-aligned, kbd-style.
    pub shortcut: Option<KbdChord>,
    /// Greyed out and non-activatable.
    pub disabled: bool,
}

impl CommandItemSpec {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: String::new(),
            shortcut: None,
            disabled: false,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    #[must_use]
    pub fn shortcut(mut self, chord: impl Into<KbdChord>) -> Self {
        self.shortcut = Some(chord.into());
        self
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// A named group of items. Maps 1:1 to shadcn's `<CommandGroup>` —
/// the heading is rendered at the top, items beneath.
#[derive(Clone, Debug)]
pub struct CommandSection {
    pub heading: String,
    pub items: Vec<CommandItemSpec>,
}

impl CommandSection {
    pub fn new(heading: impl Into<String>) -> Self {
        Self {
            heading: heading.into(),
            items: Vec::new(),
        }
    }

    #[must_use]
    pub fn item(mut self, item: CommandItemSpec) -> Self {
        self.items.push(item);
        self
    }

    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = CommandItemSpec>) -> Self {
        self.items.extend(items);
        self
    }
}

// ---------------------------------------------------------------------------
// ECS markers (mirror shadcn's `data-slot` tree shape).
// ---------------------------------------------------------------------------

/// Root of a command palette subtree. Carries the active selection
/// index for keyboard navigation.
#[derive(Component, Debug)]
pub struct Command;

/// Tracks the currently-highlighted item (by entity, not index, since
/// filtering hides items dynamically and an index would shift).
#[derive(Component, Default, Debug)]
pub struct CommandSelection {
    pub selected: Option<Entity>,
}

/// Wraps the input row at the top of the palette (search icon + text
/// field). The text field itself is a tempera `TextInputNode` child.
#[derive(Component, Debug)]
pub struct CommandInputRow;

/// Marker on the scrollable list region beneath the input.
#[derive(Component, Debug)]
pub struct CommandList;

/// Marker on a section heading (the small grey label above each
/// group's items).
#[derive(Component, Debug)]
pub struct CommandGroupHeading;

/// Marker on a section wrapper Node. Used by the filter system to
/// hide an entire section whose items have all been filtered out.
#[derive(Component, Debug)]
pub struct CommandGroup;

/// Marker on an individual command item. Carries the spec's `id` so
/// the activate observer can emit the right event.
#[derive(Component, Clone, Debug)]
pub struct CommandItem {
    pub id: String,
    pub disabled: bool,
    /// Lowercased label, cached for the filter system so it doesn't
    /// re-allocate on every keystroke.
    pub(crate) search_text: String,
}

/// Empty-state placeholder shown when the filter matches no items.
/// Toggled visible by the filter system.
#[derive(Component, Debug)]
pub struct CommandEmpty;

/// Fired (as a Bevy event-target observer) when the user activates an
/// item. Listen on the [`Command`] root entity:
///
/// ```ignore
/// commands.entity(palette).observe(|on: On<CommandActivated>| {
///     info!("activated: {}", on.id);
/// });
/// ```
#[derive(bevy::ecs::event::EntityEvent, Clone, Debug)]
pub struct CommandActivated {
    #[event_target]
    pub palette: Entity,
    pub id: String,
}
