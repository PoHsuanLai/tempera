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

More will follow as the frontend is modularized — dock and layout, settings,
panel primitives. Each lands as its own reviewable unit rather than as one
sweeping extraction.

## Design notes

Two decisions shape everything here and are worth stating up front.

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
