# tempera

Theme-aware widgets for `bevy_ui`, and the application shell built out of them.

Two layers, one workspace:

| crate | what it owns |
| --- | --- |
| `tempera-widgets` | buttons, dialogs, menus, selects, sliders, rows — the drawing |
| `tempera-input` | keybinds, commands, conditions, and dispatch |
| `tempera-dock` | panes, splits, dividers, resize, and layout persistence |
| `tempera-tree` | hierarchy, expansion, filtering, and indent — for any tree view |
| `tempera-settings` | a tabbed settings dialog — the chrome, not what goes in it |

`tempera-widgets` publishes its library as `tempera`, so a use site reads
`tempera::button` rather than `tempera_widgets::button`. The package name says
which crate of the workspace it is; the library name says what you are calling.

## The line the layers hold

A widget knows how to draw itself and nothing about who asked. A shell crate
knows what an application *shape* is — that panes divide, that commands have
conditions, that a tree collapses — and still nothing about any particular
application.

**Neither may name a specific app.** The shell crates came from the
[dawai](https://github.com/PoHsuanLai/dawai) repo, where that rule was enforced
by a cargo workspace `exclude`: a shell crate could not depend on `dawai-*`
because those packages were not members. Living in a separate repository is the
stronger form of the same rule — there is no lint to configure and no
convention to remember, because the dependency cannot resolve.

The practical test:

```sh
cargo test --workspace --lib --tests
```

This must pass with no application checked out at all.

> **Note** — four examples (`context_menu`, `singletons`, `tabs_dropdown`,
> `toast`) do not currently compile, against `ToastId` / `ToastManager` and
> `KbdChord: From<&str>`. Pre-existing and unrelated to the shell crates; hence
> `--lib --tests` above.

## Design notes

A few decisions shape everything here and are worth stating up front.

### tempera-input

**Commands are entities.** The components tempera interprets are `CommandId`,
`CommandLabel`, `Keybind`, `OnPress`, and the optional `OnRelease` / `When` /
`Priority`. An application attaches its own components to the same entity for
anything tempera has no business modelling — a palette grouping, an icon, an
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

### tempera-dock

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
*ignorable* rather than wrong — the same doctrine `tempera-input` applies to a
saved keybind naming a command that no longer exists.

**No floating panes.** Overlays are real, but they need z-order, viewport
clamping and their own drag behaviour, none of which is a split; modelling them
in the tree would mean two layout modes forever. A pane frame is `relative` +
`clip` instead, so a host anchors absolutely-positioned chrome to whichever pane
it likes. This is also what keeps resize *positional* — a divider finds its
neighbours by index among its parent's children, never by marker — which is what
makes a future drag-to-rearrange nearly free.

**Several contents in one pane are not a node kind.** A pane can hold N pages
with one showing — what a tab bar, a mode switcher and a stacked view all reduce
to — but there is no `DockTree::Tabs` variant and there will not be one. The
tree answers exactly one question, *how does the window divide*, and switching
page divides nothing: no divider moves, nothing resizes, the split structure is
identical either side. So pages are components on a pane's children, the tree is
untouched, and the layout format does not move.

`ActivePage` lives on the pane rather than in a resource, so two panes holding
pages are independent — and the crate draws no chooser. `PageLabel`, `PageIcon`
and `PageOrder` exist so whatever *does* draw one has the metadata, whether that
is a tab strip, a sidebar of icons, or a keybind with no UI at all. Same split
GTK makes between `Stack` and `StackSwitcher`.

This replaced an earlier `center_mode` module that hardcoded both "the swappable
surface is the center one" and "there is exactly one of them". Neither is a
property of a shell.

Drag-to-rearrange is not built, and that one *is* a tree change. The shape is
paid for: resize is positional, so reordering a split's children needs no
divider bookkeeping; a pane frame is a distinct entity from its content, so a
move is one reparent with the content following; and `DockCommands::move_pane`
is already the tree surgery a drop would call.

### tempera-tree

**It computes a flat list; it spawns nothing.** `visible_rows` is a pure function
returning rows in render order — no `World`, no `Commands`, no entity created.
That is how real tree views work: VS Code's explorer, Chrome DevTools' DOM tree
and every file manager keep a flat visible array and splice collapsed subtrees
*out* of it, rather than keeping a container element per group and hiding it. A
collapsed folder should cost nothing, not one UI node per file inside it. The
trade is stated plainly in the module docs — toggling a group re-emits the list,
so rows respawn where a container model would have flipped a flag.

**Items are discovered, not declared** — the opposite of `tempera-dock`, and for
a concrete reason. A dock layout *is* authored: a human writes ten panes and it
round-trips to disk. A browser tree is minted by a filesystem walk and extended
by plugin scans that finish after the window is up. So hierarchy is read off
components a host's scanner writes, and only *expansion* persists. The cost is
paid honestly: unlike `DockLayout::validate`, a duplicate group id or a parent
cycle is detectable only while walking, so the offending row is warned about and
dropped rather than caught early.

**Expansion is stored as deviations from the declared default**, in two sets and
not one. A single set of open ids carries one bit per group where the question
needs two — *is it open*, and *did the user say so*. Without the second, a
default-open group is inexpressible, which is exactly why the browser this
replaced grew a second state mechanism with the opposite default and hand-written
match arms per section. Here a section is just a root group.

Keyboard navigation is genuinely missing, not refused.

### tempera-settings

**Tabs are declared, not enumerated.** The dialog this replaced had a closed
six-variant enum, and adding a tab meant editing four places that had to stay in
lockstep — the enum, a label table, a `SystemParam` with six named queries, and
two hand-written six-tuples. Miss one and tab switching broke silently for
*every* tab, because the six-way destructure bailed. It also could not work
here: a crate that names `Audio` and `Extensions` knows it is running a DAW. A
tab is now an entity, ordered by `TabOrder`, which means an **extension** can
contribute one — something the enum made impossible.

**Content finds its body; the dialog never pushes.** Same inversion as the dock:
a panel queries by string id and parents itself in, so a tab's content can live
in a crate this one has never heard of.

**It owns the chrome and nothing else.** Not the preference values (a control's
write-back names a host type, so the binding is the host's), not persistence
(there is nothing to persist but the active tab), not the form rows
(`tempera::setting_row` and `tempera::list_row`), and not opening the dialog —
`SettingsOpen` is mirrored onto `Visibility` and never set here. Closing is
*reported* rather than swallowed, preserving tempera's deliberately
source-of-truth-agnostic `DialogDismissed`.

The title-bar height is the detail worth noting: the old code carried
`const TITLE_BAR_HEIGHT = 44.0`, a hand-copy of a tempera internal that nothing
kept in step, and subtracted it to size the content row. Here the row is
`flex_grow: 1.0`, so the number never has to be known.
