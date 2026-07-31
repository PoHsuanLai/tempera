# shellie

A reusable, content-agnostic application shell for Bevy.

shellie is to the UI what `tutti` is to audio: a self-contained set of crates
that knows nothing about dawai. The shell it provides is the kind an IDE, a
graphics editor, or a DAW would all want — keybinds, commands, docking, panels —
with none of the vocabulary of any one of them.

## Why it is a separate workspace

`crates/shellie` is `exclude`d from the app workspace, and **that exclusion is
the layering enforcement**. A crate in here physically cannot name `dawai-*`,
because those packages are not members over here. There is no lint to configure
and no convention to remember: the dependency simply will not resolve.

The practical test, which CI should run:

```sh
cargo test --manifest-path crates/shellie/Cargo.toml --workspace
```

This must pass regardless of what state the app workspace is in. It already
earned its keep once: PR #77 ejected nine crates from the app workspace,
including `dawai-frontend`, and shellie was untouched.

## Crates

| crate | what it owns |
| --- | --- |
| `shellie-input` | keybinds, commands, conditions, and dispatch |
| `shellie-dock` | panes, splits, dividers, resize, and layout persistence |

More will follow as the frontend is modularized — settings, panel primitives,
icons. Each lands as its own reviewable unit rather than as one sweeping
extraction.

## Design notes

A few decisions shape everything here and are worth stating up front.

### shellie-input

**Commands are entities.** The components shellie interprets are `CommandId`,
`CommandLabel`, `Keybind`, `OnPress`, and the optional `OnRelease` / `When` /
`Priority`. An application attaches its own components to the same entity for
anything shellie has no business modelling — a palette grouping, an icon, an
owning extension id — and queries them itself. Optional behaviour is an *absent
component*, never an `Option` field, which is why adding conditions to the model
did not grow the registration call another positional argument.

**Conditions, not focus scopes.** The obvious design — the focused widget owns a
scope, and scopes shadow each other — cannot express the cases that matter. In
VS Code, `Escape` closes the completion popup, else collapses the multi-cursor,
else closes the find bar, else leaves the terminal: four handlers reachable from
one focused widget, separated by *application state*. A `When` condition is an
ordinary Bevy run condition, so `.and()` / `not()` compose for free and the app
writes typed systems instead of strings in a bespoke expression language.

That choice is also forced by the ecosystem. leafwing reads
`ButtonInput<KeyCode>` — a global state resource — while Bevy's focus bubbling
dispatches a stream of events; by the time leafwing sees a key, the routing
information focus would filter on is gone, and chords inherently need state
rather than edges. Bevy's own discussion of this
([bevyengine/bevy#15374](https://github.com/bevyengine/bevy/discussions/15374))
has been open since 2024 with no shipped answer.

### shellie-dock

**The tree is declared, not spawned.** `DockLayout` is plain serializable data
with no `Entity` in it, so one type serves three jobs that would otherwise drift
apart: a host declares a layout, a saved layout deserializes into it, and a
runtime rearrangement edits it in place. The build then *reconciles* by `PaneId`
rather than re-spawning — a pane that survives a layout change keeps its entity,
and with it whatever content another crate parented in. Rebuilding the world on
every resize would work exactly once.

**Content finds its pane; the dock never pushes.** A panel queries for its pane
by string id and parents itself in. That inversion is what lets a panel live in a
crate the dock has never heard of, and it is why an unrecognised id is
*ignorable* rather than wrong — the same doctrine `shellie-input` applies to a
saved keybind naming a command that no longer exists.

**No floating panes.** Overlays are real, but they need z-order, viewport
clamping and their own drag behaviour, none of which is a split; modelling them
in the tree would mean two layout modes forever. A pane frame is `relative` +
`clip` instead, so a host anchors absolutely-positioned chrome to whichever pane
it likes. This is also what keeps resize *positional* — a divider finds its
neighbours by index among its parent's children, never by marker — which is what
makes a future drag-to-rearrange nearly free.

Tabs are not built. The shape is paid for: the persisted tree is recursive from
v1 so a `Tabs` variant is additive, a pane frame is a distinct entity from its
content so a tab-move is one reparent, and `DockCommands::move_pane` is already
the tree surgery a drop would call.
