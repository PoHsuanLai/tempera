//! Declaring menu items as entities, so several crates can contribute to
//! one menu.
//!
//! [`super::OpenContextMenu`] takes a `Vec<MenuItemSpec>` — fine when one
//! place knows every entry, useless when they arrive from a plugin, an
//! extension, or a feature module the menu has never heard of. This is the
//! other half: items are **entities**, tagged with the surface they belong
//! to, collected on open, and lowered into the specs the renderer already
//! takes.
//!
//! ```ignore
//! use tempera::context_menu::{menu_item, AppMenuExt, Destructive, MenuOrder, VisibleWhen};
//!
//! app.spawn_menu_item((
//!     menu_item("timeline.clip", "Delete"),
//!     MenuOrder(90),
//!     Destructive,
//!     VisibleWhen::new(|s: Res<Selection>| !s.clips.is_empty()),
//! ));
//! ```
//!
//! Activation is *reported*, never invoked here: the renderer already
//! fires [`super::MenuItemActivated`] carrying the item's own entity, so a
//! host observes that and decides. A closure component would be a second
//! answer to a question already answered — and one the scheduler cannot
//! see into, parallelize, or query.
//!
//! # Surfaces
//!
//! A *surface* is a string naming one menu — `"timeline.clip"`,
//! `"browser.row.audio"`. Not an enum, so a crate can introduce its own
//! without touching this module, and not `&'static str`, so a surface id
//! read from an extension manifest does not have to be leaked to be used.
//!
//! # Submenus are children
//!
//! A nested item is an ordinary item entity with `ChildOf(parent)`. It
//! carries the same components, gets gated by the same [`VisibleWhen`],
//! and is reported by the same [`super::MenuItemActivated`] with its own
//! entity — so a submenu costs no vocabulary at all.
//!
//! Children that depend on live state (the effects on the *selected*
//! track) are the host's to reconcile, the same way it would reconcile
//! any other list of entities. This module deliberately offers no
//! "build children from a closure" hook: it would be a second path
//! through which items appear, and the one thing it can do that spawning
//! cannot — react instantly to state — is exactly what a reconciler
//! already does.
//!
//! # Optional is an absent component
//!
//! No [`MenuOrder`] means last; no [`VisibleWhen`] means always; neither
//! [`MenuShortcut`] nor [`MenuShortcutFor`] means no keycap. That is why
//! gaining conditions did not grow anything an extra field.

use std::borrow::Cow;

use bevy::ecs::schedule::{BoxedCondition, SystemCondition};
use bevy::prelude::*;

use crate::kbd::KbdChord;

use super::request::{MenuItemSpec, MenuRequest};
use super::{MenuRootMarker, OpenContextMenu};

// ── identity ─────────────────────────────────────────────────────────────

/// The surface this item belongs to — which menu it appears in.
///
/// Present on **root** items only. A child's membership comes from its
/// parent via `ChildOf`; see the module docs.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MenuSurface(pub String);

impl MenuSurface {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MenuSurface {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// The text on the row.
///
/// Required, not an override. An item whose label had to be looked up
/// elsewhere is an item that can fail to have one, and the only thing to
/// do at paint time with a missing label is render an apology into the
/// user's menu.
#[derive(Component, Clone, Debug)]
pub struct MenuLabel(pub Cow<'static, str>);

impl MenuLabel {
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self(label.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Sort key within a menu — lowest first. Absent sorts last.
///
/// Ties break by entity, which is stable within a run but not across
/// them; give items that must hold an order an explicit one.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MenuOrder(pub i32);

// ── presentation ─────────────────────────────────────────────────────────

/// Render the row in the destructive (red) style.
#[derive(Component, Default, Debug)]
pub struct Destructive;

/// Draw a separator above this row.
///
/// Suppressed when the item ends up first among the visible ones, so a
/// separator cannot lead a menu just because whatever preceded it was
/// conditioned out.
#[derive(Component, Default, Debug)]
pub struct SeparatorBefore;

/// A fixed keycap shown at the trailing edge.
///
/// Literal: whatever the caller wrote, unchanged for the life of the item. For
/// a chord the user can rebind, declare [`MenuShortcutFor`] instead.
#[derive(Component, Clone, Debug)]
pub struct MenuShortcut(pub KbdChord);

/// Show the live binding of `0` as this row's keycap.
///
/// Resolved during [`collect_surface`] — every time the menu opens, from the
/// command's current [`Keybind`](tempera_input::Keybind) — so a rebind shows
/// up the next time the menu is opened, and an unbind removes the keycap
/// rather than leaving a stale one.
///
/// # Why not just store the chord
///
/// A [`MenuShortcut`] written at registration is a second owner of a value
/// `Keybind` already owns, and nothing updates it: the keycap shows whatever
/// was bound the moment the item was declared, for the life of the app. Same
/// reasoning as [`TooltipShortcutFor`](crate::tooltip::TooltipShortcutFor),
/// same remedy — name the command, resolve at the point of use. Here the point
/// of use is `collect_surface`, which already has `&mut World`, so the lookup
/// costs a hash probe per row per open.
///
/// Takes precedence over [`MenuShortcut`] when both are present: the live
/// answer is the truer one.
///
/// An id nothing claims shows no keycap. Commands are registered by whichever
/// crates are present, so a menu naming one from an absent crate is ordinary
/// rather than broken.
#[derive(Component, Clone, Debug)]
pub struct MenuShortcutFor(pub tempera_input::CommandId);

/// Render the row greyed and unclickable.
#[derive(Component, Default, Debug)]
pub struct MenuDisabled;

// ── gating ───────────────────────────────────────────────────────────────

/// Show this item only when the condition holds. Absent means always.
///
/// A condition is an ordinary Bevy run condition — any read-only system
/// returning `bool` — so `.and()`, `.or()` and `not()` compose for free
/// and the host writes typed systems rather than strings in a bespoke
/// expression language.
///
/// **It is boxed rather than a bare `fn` pointer, and that is the whole
/// point.** A `fn(&World) -> bool` cannot capture, so an item whose
/// visibility depends on anything known only at spawn time — a plugin id,
/// a row index, a predicate parsed from a manifest — cannot express it,
/// and the host ends up adding a *second* predicate component beside this
/// one. Capture removes the reason for that second mechanism to exist.
///
/// ```ignore
/// VisibleWhen::new(is_playing.and(not(has_selection)))
/// ```
#[derive(Component)]
pub struct VisibleWhen {
    condition: BoxedCondition,
    initialized: bool,
}

impl VisibleWhen {
    /// Wrap a Bevy run condition.
    pub fn new<M>(condition: impl SystemCondition<M>) -> Self {
        Self {
            condition: Box::new(IntoSystem::into_system(condition)),
            initialized: false,
        }
    }

    /// Resolve the condition's system parameters. Idempotent.
    ///
    /// Runs at collection time, where `&mut World` is available. Skipping
    /// it panics inside Bevy, so [`eval`](Self::eval) refuses instead.
    fn initialize(&mut self, world: &mut World) {
        if !self.initialized {
            self.condition.initialize(world);
            self.initialized = true;
        }
    }

    /// Evaluate against the current world.
    ///
    /// A condition that cannot be run returns `false`. A gate that cannot
    /// be evaluated must not open — showing an item whose precondition is
    /// unknown is how a "Delete" lands in a menu that has nothing selected.
    fn eval(&mut self, world: &World) -> bool {
        if !self.initialized {
            error!("[tempera] menu condition evaluated before initialize(); treating as false");
            return false;
        }
        match self.condition.run_readonly((), world) {
            Ok(passed) => passed,
            Err(e) => {
                error!("[tempera] menu condition failed to run: {e}; treating as false");
                false
            }
        }
    }
}

// ── spawning ─────────────────────────────────────────────────────────────

/// Marker on every registry item, so a menu's items can be found without
/// naming any other component.
#[derive(Component, Default, Debug)]
pub struct MenuItemMarker;

/// The components every root item needs: a surface and a label.
///
/// ```ignore
/// commands.spawn((menu_item("timeline.clip", "Rename"), MenuOrder(10)));
/// ```
pub fn menu_item(surface: impl Into<String>, label: impl Into<Cow<'static, str>>) -> impl Bundle {
    (
        MenuItemMarker,
        MenuSurface(surface.into()),
        MenuLabel::new(label),
    )
}

/// The components a **child** item needs. Membership comes from its
/// parent, so there is no surface here.
///
/// ```ignore
/// commands.spawn((child_item("Sine"), ChildOf(parent)));
/// ```
pub fn child_item(label: impl Into<Cow<'static, str>>) -> impl Bundle {
    (MenuItemMarker, MenuLabel::new(label))
}

/// Registration entry point.
pub trait AppMenuExt {
    /// Spawn a menu item, naming it after its label for the inspector.
    fn spawn_menu_item(&mut self, bundle: impl Bundle) -> &mut Self;
}

impl AppMenuExt for App {
    fn spawn_menu_item(&mut self, bundle: impl Bundle) -> &mut Self {
        let world = self.world_mut();
        let entity = world.spawn(bundle).id();
        if let Some(label) = world.get::<MenuLabel>(entity).map(|l| l.0.clone()) {
            world
                .entity_mut(entity)
                .insert(Name::new(format!("menu:{label}")));
        } else {
            error!(
                "[tempera] spawn_menu_item called with a bundle carrying no MenuLabel; \
                 despawning. Wrap it in menu_item(surface, label) or child_item(label)."
            );
            world.entity_mut(entity).despawn();
        }
        self
    }
}

// ── collection ───────────────────────────────────────────────────────────

/// Open the menu for `surface` at `anchor` (window-space pixels).
///
/// Collects the surface's items, drops the ones whose [`VisibleWhen`]
/// does not hold, sorts by [`MenuOrder`], and hands the result to the
/// renderer. A surface with nothing visible opens no menu at all, rather
/// than an empty box.
pub fn open_surface(world: &mut World, surface: &str, anchor: Vec2) {
    let items = collect_surface(world, surface);
    if items.is_empty() {
        return;
    }
    world.write_message(OpenContextMenu(MenuRequest { anchor, items }));
}

/// Collect and lower one surface's visible items. Exposed for tests and
/// for a host that wants the specs without opening anything.
pub fn collect_surface(world: &mut World, surface: &str) -> Vec<MenuItemSpec> {
    let roots: Vec<Entity> = world
        .query_filtered::<(Entity, &MenuSurface), With<MenuItemMarker>>()
        .iter(world)
        .filter(|(_, s)| s.as_str() == surface)
        .map(|(e, _)| e)
        .collect();

    let specs = resolve_level(world, roots);
    suppress_leading_separator(specs)
}

/// Resolve one level of items: gate, sort, lower, and recurse into
/// children.
fn resolve_level(world: &mut World, entities: Vec<Entity>) -> Vec<MenuItemSpec> {
    let mut visible: Vec<(MenuOrder, Entity)> = Vec::new();
    for entity in entities {
        if !passes(world, entity) {
            continue;
        }
        let order = world
            .get::<MenuOrder>(entity)
            .copied()
            .unwrap_or(MenuOrder(i32::MAX));
        visible.push((order, entity));
    }
    // Entity is the tie-break so the order is at least stable within a
    // run; items that must hold a relative order declare one.
    visible.sort_by_key(|(order, entity)| (*order, *entity));

    visible
        .into_iter()
        .map(|(_, entity)| lower(world, entity))
        .collect()
}

/// Evaluate an item's gate, initializing it on first use.
///
/// The gate is taken off the entity, run, and put back. A condition is a
/// `System`, so running it needs `&mut` on the condition and `&World` at
/// once — which it cannot have while still borrowed from that same world.
/// Detaching for the duration is what makes the two borrows disjoint.
fn passes(world: &mut World, entity: Entity) -> bool {
    let Some(mut gate) = world.entity_mut(entity).take::<VisibleWhen>() else {
        return true;
    };
    gate.initialize(world);
    let passed = gate.eval(world);
    world.entity_mut(entity).insert(gate);
    passed
}

/// The keycap for a row: the live binding if it tracks a command, else the
/// literal chord, else nothing.
///
/// Read here rather than stored on the item, so a rebind between two openings
/// of the same menu shows the new chord. A command the registry does not know —
/// or one with no binding — yields `None`, which renders no keycap.
fn resolve_shortcut(world: &World, entity: Entity) -> Option<KbdChord> {
    if let Some(tracked) = world.get::<MenuShortcutFor>(entity) {
        let live = world
            .get_resource::<tempera_input::CommandRegistry>()
            .and_then(|registry| registry.get(tracked.0.as_str()))
            .and_then(|command| world.get::<tempera_input::Keybind>(command))
            .map(|bind| bind.0.clone().into());
        // A tracked row shows the live answer or nothing — never a stale
        // literal, which is the staleness this component exists to avoid.
        return live;
    }
    world.get::<MenuShortcut>(entity).map(|s| s.0.clone())
}

/// Turn one item entity into a spec, recursing into its children.
fn lower(world: &mut World, entity: Entity) -> MenuItemSpec {
    let label = world
        .get::<MenuLabel>(entity)
        .map(|l| l.0.to_string())
        .unwrap_or_default();
    let destructive = world.get::<Destructive>(entity).is_some();
    let separator_before = world.get::<SeparatorBefore>(entity).is_some();
    let disabled = world.get::<MenuDisabled>(entity).is_some();
    let shortcut = resolve_shortcut(world, entity);

    // The id is what a host matches on when it routes by string rather
    // than by entity. Both travel on `MenuItemActivated`.
    let mut spec = MenuItemSpec::new(format!("{entity}"))
        .label(label)
        .origin(entity);
    if destructive {
        spec = spec.destructive();
    }
    if separator_before {
        spec = spec.separator_before();
    }
    if disabled {
        spec = spec.disabled();
    }
    if let Some(chord) = shortcut {
        spec = spec.shortcut(chord);
    }

    let kids: Vec<Entity> = world
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    let kids: Vec<Entity> = kids
        .into_iter()
        .filter(|e| world.get::<MenuItemMarker>(*e).is_some())
        .collect();
    if !kids.is_empty() {
        let children = suppress_leading_separator(resolve_level(world, kids));
        spec = spec.children(children);
    }

    spec
}

/// Drop a separator on the first visible row.
///
/// Conditions decide what survives, so the item a separator was authored
/// to sit below may not be there — and a menu that opens with a rule
/// across its top looks broken.
fn suppress_leading_separator(mut specs: Vec<MenuItemSpec>) -> Vec<MenuItemSpec> {
    if let Some(first) = specs.first_mut() {
        first.separator_before = false;
    }
    specs
}

// ── close reporting ──────────────────────────────────────────────────────

/// Message: a context menu closed, for any reason.
///
/// Dismissal happens through several paths — Escape, activation, a click
/// outside, focus loss — and a host that stashed state for the menu
/// (which row was right-clicked) needs one place to clear it. This is
/// that place. It reports; it does not ask.
#[derive(Message, Debug, Clone, Copy)]
pub struct MenuClosed;

/// Fire [`MenuClosed`] when a menu root goes away.
///
/// Keyed on the root's removal rather than on each dismissal path, so a
/// path added later reports without being taught to.
pub(crate) fn report_menu_closed(
    _remove: On<Remove, MenuRootMarker>,
    mut writer: MessageWriter<MenuClosed>,
) {
    writer.write(MenuClosed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct HasSelection(bool);

    fn has_selection(s: Res<HasSelection>) -> bool {
        s.0
    }

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<HasSelection>();
        world
    }

    fn labels(specs: &[MenuItemSpec]) -> Vec<&str> {
        specs.iter().map(|s| s.label.as_str()).collect()
    }

    #[test]
    fn items_are_collected_by_surface() {
        let mut w = world();
        w.spawn(menu_item("timeline.clip", "Rename"));
        w.spawn(menu_item("timeline.clip", "Delete"));
        w.spawn(menu_item("browser.row", "Reveal"));

        let clip = collect_surface(&mut w, "timeline.clip");
        assert_eq!(clip.len(), 2);
        let browser = collect_surface(&mut w, "browser.row");
        assert_eq!(labels(&browser), ["Reveal"]);
    }

    #[test]
    fn an_unknown_surface_yields_nothing() {
        let mut w = world();
        w.spawn(menu_item("timeline.clip", "Rename"));
        assert!(collect_surface(&mut w, "nobody.declared.this").is_empty());
    }

    #[test]
    fn order_sorts_and_absent_sorts_last() {
        let mut w = world();
        w.spawn((menu_item("s", "unordered"),));
        w.spawn((menu_item("s", "second"), MenuOrder(20)));
        w.spawn((menu_item("s", "first"), MenuOrder(10)));

        assert_eq!(
            labels(&collect_surface(&mut w, "s")),
            ["first", "second", "unordered"]
        );
    }

    #[test]
    fn a_condition_gates_the_item() {
        let mut w = world();
        w.spawn((menu_item("s", "always"), MenuOrder(0)));
        w.spawn((
            menu_item("s", "needs selection"),
            MenuOrder(1),
            VisibleWhen::new(has_selection),
        ));

        assert_eq!(labels(&collect_surface(&mut w, "s")), ["always"]);

        w.resource_mut::<HasSelection>().0 = true;
        assert_eq!(
            labels(&collect_surface(&mut w, "s")),
            ["always", "needs selection"]
        );
    }

    #[test]
    fn conditions_compose_with_the_standard_combinators() {
        let mut w = world();
        w.spawn((
            menu_item("s", "nothing selected"),
            VisibleWhen::new(not(has_selection)),
        ));

        assert_eq!(labels(&collect_surface(&mut w, "s")), ["nothing selected"]);
        w.resource_mut::<HasSelection>().0 = true;
        assert!(collect_surface(&mut w, "s").is_empty());
    }

    #[test]
    fn a_capturing_condition_works() {
        // The reason the gate is boxed rather than a `fn` pointer: an
        // item whose visibility depends on something known only at spawn
        // time cannot be expressed by a non-capturing function, and a
        // host that needs one ends up adding a second predicate
        // component beside this one.
        let mut w = world();
        for wanted in [true, false] {
            w.spawn((
                menu_item("s", if wanted { "wants true" } else { "wants false" }),
                VisibleWhen::new(move |s: Res<HasSelection>| s.0 == wanted),
            ));
        }

        assert_eq!(labels(&collect_surface(&mut w, "s")), ["wants false"]);
        w.resource_mut::<HasSelection>().0 = true;
        assert_eq!(labels(&collect_surface(&mut w, "s")), ["wants true"]);
    }

    #[test]
    fn a_condition_survives_repeated_collection() {
        // The gate is detached and re-attached to evaluate it; a second
        // open must find it still there and still working.
        let mut w = world();
        w.spawn((menu_item("s", "gated"), VisibleWhen::new(has_selection)));

        assert!(collect_surface(&mut w, "s").is_empty());
        w.resource_mut::<HasSelection>().0 = true;
        assert_eq!(labels(&collect_surface(&mut w, "s")), ["gated"]);
        assert_eq!(labels(&collect_surface(&mut w, "s")), ["gated"]);
    }

    #[test]
    fn presentation_markers_reach_the_spec() {
        let mut w = world();
        w.spawn((menu_item("s", "Delete"), Destructive, MenuDisabled));

        let specs = collect_surface(&mut w, "s");
        assert!(specs[0].destructive);
        assert!(!specs[0].enabled, "MenuDisabled must disable the row");
    }

    #[test]
    fn the_origin_entity_travels_so_activation_can_be_routed() {
        // Pure reporting rests on this: the host observes
        // `MenuItemActivated` and needs the declaring entity back to know
        // what was clicked.
        let mut w = world();
        let entity = w.spawn(menu_item("s", "Rename")).id();

        assert_eq!(collect_surface(&mut w, "s")[0].origin, Some(entity));
    }

    #[test]
    fn children_become_a_submenu() {
        let mut w = world();
        let parent = w.spawn((menu_item("s", "Add LFO"),)).id();
        w.spawn((child_item("Sine"), MenuOrder(0), ChildOf(parent)));
        w.spawn((child_item("Square"), MenuOrder(1), ChildOf(parent)));

        let specs = collect_surface(&mut w, "s");
        assert_eq!(specs.len(), 1, "the child is not a root");
        assert_eq!(labels(&specs[0].children), ["Sine", "Square"]);
    }

    #[test]
    fn a_child_can_be_gated_independently() {
        // What the closure-built children could never do: a
        // `Vec<MenuItemSpec>` has nowhere to hang a condition.
        let mut w = world();
        let parent = w.spawn((menu_item("s", "Add"),)).id();
        w.spawn((child_item("always"), MenuOrder(0), ChildOf(parent)));
        w.spawn((
            child_item("gated"),
            MenuOrder(1),
            VisibleWhen::new(has_selection),
            ChildOf(parent),
        ));

        let specs = collect_surface(&mut w, "s");
        assert_eq!(labels(&specs[0].children), ["always"]);

        w.resource_mut::<HasSelection>().0 = true;
        let specs = collect_surface(&mut w, "s");
        assert_eq!(labels(&specs[0].children), ["always", "gated"]);
    }

    #[test]
    fn submenus_nest_arbitrarily() {
        let mut w = world();
        let root = w.spawn((menu_item("s", "Add"),)).id();
        let mid = w.spawn((child_item("Automation"), ChildOf(root))).id();
        w.spawn((child_item("Volume"), ChildOf(mid)));

        let specs = collect_surface(&mut w, "s");
        assert_eq!(labels(&specs[0].children[0].children), ["Volume"]);
    }

    #[test]
    fn a_non_item_child_is_not_a_menu_row() {
        // Items are ordinary entities, so a host may well parent
        // something else under one — an icon, a bit of its own state.
        let mut w = world();
        let parent = w.spawn((menu_item("s", "Add"),)).id();
        w.spawn((child_item("Sine"), ChildOf(parent)));
        w.spawn(ChildOf(parent));

        let specs = collect_surface(&mut w, "s");
        assert_eq!(labels(&specs[0].children), ["Sine"]);
    }

    #[test]
    fn a_gated_parent_hides_its_whole_submenu() {
        let mut w = world();
        let parent = w
            .spawn((menu_item("s", "Add"), VisibleWhen::new(has_selection)))
            .id();
        w.spawn((child_item("Sine"), ChildOf(parent)));

        assert!(collect_surface(&mut w, "s").is_empty());
    }

    #[test]
    fn a_separator_never_leads_a_menu() {
        // The item a separator sat below can be conditioned out, and a
        // menu opening with a rule across its top looks broken.
        let mut w = world();
        w.spawn((
            menu_item("s", "conditional"),
            MenuOrder(0),
            VisibleWhen::new(has_selection),
        ));
        w.spawn((menu_item("s", "Delete"), MenuOrder(1), SeparatorBefore));

        let specs = collect_surface(&mut w, "s");
        assert_eq!(labels(&specs), ["Delete"]);
        assert!(
            !specs[0].separator_before,
            "a separator on the first visible row must be suppressed"
        );

        w.resource_mut::<HasSelection>().0 = true;
        let specs = collect_surface(&mut w, "s");
        assert!(
            specs[1].separator_before,
            "and must survive when something precedes it"
        );
    }

    #[test]
    fn a_separator_is_suppressed_inside_a_submenu_too() {
        let mut w = world();
        let parent = w.spawn((menu_item("s", "Add"),)).id();
        w.spawn((child_item("Sine"), SeparatorBefore, ChildOf(parent)));

        let specs = collect_surface(&mut w, "s");
        assert!(!specs[0].children[0].separator_before);
    }

    #[test]
    fn spawn_menu_item_names_the_entity_and_rejects_a_label_less_bundle() {
        let mut app = App::new();
        app.spawn_menu_item(menu_item("s", "Rename"));
        app.spawn_menu_item((MenuItemMarker, MenuOrder(0)));

        let mut q = app.world_mut().query::<(&Name, &MenuLabel)>();
        let names: Vec<String> = q.iter(app.world()).map(|(n, _)| n.to_string()).collect();
        assert_eq!(names, ["menu:Rename"]);

        let mut all = app.world_mut().query::<&MenuItemMarker>();
        assert_eq!(
            all.iter(app.world()).count(),
            1,
            "the rejected entity must not be left behind"
        );
    }

    #[test]
    fn closing_a_menu_is_reported_however_it_closed() {
        // A host that stashed "which row was right-clicked" needs one
        // place to clear it. Keying on the root's removal rather than on
        // each dismissal path means a path added later reports for free.
        let mut app = App::new();
        app.add_message::<MenuClosed>()
            .add_observer(report_menu_closed);

        let root = app
            .world_mut()
            .spawn(MenuRootMarker { opened_at_frame: 0 })
            .id();
        app.update();
        assert_eq!(app.world().resource::<Messages<MenuClosed>>().len(), 0);

        app.world_mut().entity_mut(root).despawn();
        assert_eq!(
            app.world().resource::<Messages<MenuClosed>>().len(),
            1,
            "despawning the root reports a close"
        );
    }

    #[test]
    fn an_uninitialized_condition_denies_rather_than_allows() {
        // Reached only if collection is bypassed, but the direction of
        // the failure is the point: an unevaluatable gate must not put a
        // destructive verb in front of the user.
        let world = World::new();
        let mut gate = VisibleWhen::new(|| true);
        assert!(!gate.eval(&world));
    }
    // ── live shortcuts ───────────────────────────────────────────────────

    /// An app with one command registered under `id`, bound to `chord` if
    /// given, plus this crate's menu test resource.
    ///
    /// Goes through `spawn_command` rather than spawning the components by
    /// hand: that is what indexes the command in `CommandRegistry`, and a bare
    /// spawn leaves the lookup empty — every assertion here would then pass for
    /// the wrong reason, seeing "no binding" where it meant "not registered".
    fn app_with_command(id: &str, chord: Option<tempera_input::Chord>) -> App {
        use tempera_input::{AppCommandExt, CommandLabel, CommandRegistry, dyn_cmd, on_press};

        let mut app = App::new();
        app.init_resource::<HasSelection>();
        app.init_resource::<CommandRegistry>();
        app.spawn_command(dyn_cmd(id, (CommandLabel::new("cmd"), on_press(|_| {}))));

        if let Some(chord) = chord {
            let command = app
                .world()
                .resource::<CommandRegistry>()
                .get(id)
                .expect("just registered");
            app.world_mut()
                .entity_mut(command)
                .insert(tempera_input::Keybind(chord));
        }
        app
    }

    fn shortcut_glyphs(spec: &MenuItemSpec) -> Option<Vec<String>> {
        spec.shortcut
            .as_ref()
            .map(|chord| chord.render_order().map(|k| k.glyph()).collect())
    }

    #[test]
    fn a_tracked_row_shows_the_live_binding() {
        let mut app = app_with_command("edit.undo", Some(tempera_input::key(KeyCode::KeyZ)));
        let w = app.world_mut();
        w.spawn((
            menu_item("s", "Undo"),
            MenuShortcutFor(tempera_input::CommandId("edit.undo".to_owned())),
        ));

        let items = collect_surface(w, "s");
        assert_eq!(shortcut_glyphs(&items[0]), Some(vec!["Z".to_owned()]));
    }

    #[test]
    fn a_rebind_between_openings_shows_the_new_chord() {
        // The reason this component exists. A `MenuShortcut` written at
        // registration would still be showing the old chord here.
        use tempera_input::{CommandRegistry, Keybind};

        let mut app = app_with_command("edit.undo", Some(tempera_input::key(KeyCode::KeyZ)));
        let w = app.world_mut();
        w.spawn((
            menu_item("s", "Undo"),
            MenuShortcutFor(tempera_input::CommandId("edit.undo".to_owned())),
        ));
        let first = collect_surface(w, "s");
        assert_eq!(shortcut_glyphs(&first[0]), Some(vec!["Z".to_owned()]));

        let command = w
            .resource::<CommandRegistry>()
            .get("edit.undo")
            .expect("registered");
        w.entity_mut(command)
            .insert(Keybind(tempera_input::key(KeyCode::KeyY)));

        let second = collect_surface(w, "s");
        assert_eq!(shortcut_glyphs(&second[0]), Some(vec!["Y".to_owned()]));
    }

    #[test]
    fn an_unbind_between_openings_drops_the_keycap() {
        // Unbinding is a component *removal*. Resolving at open never asks
        // whether anything changed, so there is nothing to miss.
        use tempera_input::{CommandRegistry, Keybind};

        let mut app = app_with_command("edit.undo", Some(tempera_input::key(KeyCode::KeyZ)));
        let w = app.world_mut();
        w.spawn((
            menu_item("s", "Undo"),
            MenuShortcutFor(tempera_input::CommandId("edit.undo".to_owned())),
        ));
        assert!(shortcut_glyphs(&collect_surface(w, "s")[0]).is_some());

        let command = w
            .resource::<CommandRegistry>()
            .get("edit.undo")
            .expect("registered");
        w.entity_mut(command).remove::<Keybind>();

        assert_eq!(shortcut_glyphs(&collect_surface(w, "s")[0]), None);
    }

    #[test]
    fn a_command_nothing_registered_shows_no_keycap() {
        // A menu naming a command from a crate this build does not ship.
        let mut app = app_with_command("edit.undo", Some(tempera_input::key(KeyCode::KeyZ)));
        let w = app.world_mut();
        w.spawn((
            menu_item("s", "Ghost"),
            MenuShortcutFor(tempera_input::CommandId("no.such.command".to_owned())),
        ));

        let items = collect_surface(w, "s");
        assert_eq!(shortcut_glyphs(&items[0]), None);
    }

    #[test]
    fn a_literal_keycap_is_untouched() {
        // "Esc to dismiss" has no command behind it and must survive.
        let mut w = world();
        w.spawn((
            menu_item("s", "Dismiss"),
            MenuShortcut(KbdChord::key(KeyCode::Escape)),
        ));

        let items = collect_surface(&mut w, "s");
        assert_eq!(
            shortcut_glyphs(&items[0]),
            Some(vec!["\u{238b}".to_owned()])
        );
    }

    #[test]
    fn tracking_wins_over_a_literal_and_never_falls_back_to_it() {
        // Both present is a caller mistake, but it must resolve one way and
        // stay there. Falling back to the literal when a binding is missing
        // would show a chord that is not bound to anything.
        let mut app = app_with_command("edit.undo", None);
        let w = app.world_mut();
        w.spawn((
            menu_item("s", "Undo"),
            MenuShortcut(KbdChord::key(KeyCode::KeyQ)),
            MenuShortcutFor(tempera_input::CommandId("edit.undo".to_owned())),
        ));

        let items = collect_surface(w, "s");
        assert_eq!(
            shortcut_glyphs(&items[0]),
            None,
            "an unbound tracked row shows nothing, not the stale literal"
        );
    }
}
