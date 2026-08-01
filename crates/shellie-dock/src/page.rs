//! Several candidate contents in one pane, one showing at a time.
//!
//! An IDE's editor area versus its diff view; a graphics editor's canvas versus
//! its node graph; a DAW's timeline versus its piano roll. Each is one pane
//! whose *contents* change wholesale rather than by scrolling.
//!
//! ```
//! use bevy::prelude::*;
//! use shellie_dock::{ActivePage, Page, PageId, PageLabel, PageOrder};
//!
//! fn add_editor(mut commands: Commands, pane: Entity) {
//!     commands.spawn((
//!         Page,
//!         PageId::from("piano_roll"),
//!         PageLabel("Piano Roll"),
//!         PageOrder(30),
//!         ChildOf(pane),
//!     ));
//!     commands.entity(pane).insert(ActivePage::at("piano_roll"));
//! }
//! ```
//!
//! # Why this is not a node in the tree
//!
//! The obvious modelling — a `Tabs` variant of [`DockTree`](crate::DockTree)
//! beside `Pane` and `Split` — is wrong, and it took building the tree to see
//! why. [`DockTree`](crate::DockTree) answers exactly one question: **how does
//! the window divide?** Every field on it is geometric (`size`, `min_size`,
//! `visibility`) and so is every consumer.
//!
//! Switching page changes none of that. No divider moves, nothing resizes, the
//! split structure is identical before and after. The tree cannot tell the
//! difference and should not have to — so pages are components on the pane's
//! *children*, the tree is untouched, and the persisted layout format does not
//! move.
//!
//! [`Pane`](crate::Pane) already had the necessary shape: content is a **child**
//! of the pane frame, never the frame itself. A pane with three page children
//! and one active needs no new node kind.
//!
//! # Not "tabs", and not "center"
//!
//! Not **tabs**, because that names a *widget* — a clickable strip along the top
//! — and this is a data model. A host may choose its pages with a tab bar, a
//! sidebar of icons, a dropdown, a keybind, or nothing visible at all. GTK
//! splits the same way (`Stack` holds the pages, `StackSwitcher` picks one), as
//! does Qt with `QStackedWidget`.
//!
//! Not **center**, because nothing here is about being in the middle. This
//! module replaces an earlier `center_mode` that hardcoded both "the swappable
//! surface is the center one" and "there is exactly one of them" — neither of
//! which is a property of a shell. Two panes can hold pages independently, and
//! [`ActivePage`] lives on the pane precisely so they do.
//!
//! # Picking a page is the host's job
//!
//! This module says which page is active and hides the rest. It does not draw a
//! chooser, and it does not decide what a click means. [`PageLabel`],
//! [`PageIcon`] and [`PageOrder`] exist so whatever *does* draw one has the
//! metadata, without this crate knowing what that is.

use bevy::prelude::*;

/// One candidate content of a pane. Spawn it as a **child** of the pane frame.
///
/// Inserting it attaches [`PageId`], [`PageLabel`] and [`PageOrder`] at their
/// defaults through Bevy's required-components chain. At minimum override
/// [`PageId`] — the default empty string identifies nothing.
#[derive(Component, Debug, Clone, Copy, Default)]
#[require(PageId, PageLabel, PageOrder)]
pub struct Page;

/// Stable identity for a page, unique within its pane.
///
/// Owned rather than `&'static str`, matching [`PaneId`](crate::PaneId): an
/// extension can mint one at runtime. It is also the key
/// [`ActivePage`] names and the key a session save round-trips, which is why an
/// unrecognised id is survivable — see [`ActivePage::resolve`].
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageId(pub String);

impl PageId {
    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for PageId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// Human-readable name, for whatever renders a chooser.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PageLabel(pub &'static str);

/// Optional icon for a chooser.
///
/// Outside [`Page`]'s require chain deliberately: not every page has one, and
/// there is no useful default `Handle<Image>` to invent for those that do not.
#[derive(Component, Debug, Clone)]
pub struct PageIcon(pub Handle<Image>);

/// Sort key for a chooser; lower goes first.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageOrder(pub i32);

/// Which page of a pane is showing. Goes on the **pane frame**.
///
/// On the pane rather than in a resource, so two panes holding pages are
/// independent. A global "the active page" would make that unrepresentable —
/// invisible until the second such pane exists, and then structural.
///
/// `None` shows no page at all. That is the honest state before a host has
/// chosen, and it is also what a pane with no pages reads as.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivePage(pub Option<PageId>);

impl ActivePage {
    /// Start with `id` showing.
    pub fn at(id: impl Into<PageId>) -> Self {
        Self(Some(id.into()))
    }

    /// Show nothing.
    pub fn none() -> Self {
        Self(None)
    }

    /// Whether `id` is the active page.
    pub fn is(&self, id: &str) -> bool {
        self.0.as_ref().is_some_and(|active| active.as_str() == id)
    }

    /// Switch to `id`.
    pub fn set(&mut self, id: impl Into<PageId>) {
        self.0 = Some(id.into());
    }

    /// The active id, if any.
    pub fn id(&self) -> Option<&str> {
        self.0.as_ref().map(PageId::as_str)
    }

    /// The active id if `available` contains it, else the first of `available`.
    ///
    /// The tolerance a restored session needs: a save naming a page this build
    /// no longer ships must not leave the pane blank. Same doctrine as an
    /// unknown [`PaneId`](crate::PaneId) or an unknown saved keybind — an id
    /// nothing claims is ignorable, not an error.
    ///
    /// `available` is expected in the order a chooser would show, so the
    /// fallback is the first page rather than an arbitrary one.
    pub fn resolve<'a>(&self, available: &[&'a str]) -> Option<&'a str> {
        if let Some(active) = self.id()
            && let Some(found) = available.iter().find(|id| **id == active)
        {
            return Some(found);
        }
        available.first().copied()
    }
}

/// Show the active page of each pane and hide its siblings.
///
/// `Display::None`, not `Visibility::Hidden`, for the reason
/// [`PaneVisibility`](crate::PaneVisibility) gives: a hidden page must measure
/// zero. A page that owns a camera reads its own `ComputedNode` to size a
/// viewport, and a `Visibility::Hidden` node still occupies its space — so the
/// inactive page would keep claiming a rectangle it is not drawing in.
///
/// A page with no [`Node`] is left alone rather than skipped over: a host may
/// well hang non-UI pages (a camera rig, a set of systems' marker) off a pane,
/// and those have nothing to hide.
///
/// Watching `Changed<Children>` alongside `Changed<ActivePage>` is what makes a
/// page spawned *later* get hidden. `Added<Children>` is not enough — it fires
/// only when a container gains its first child ever, so a page joining a
/// settled pane would render on top of the active one until the next switch.
///
/// # Works without a dock
///
/// The queries deliberately do **not** filter on [`Pane`]: carrying an
/// [`ActivePage`] is what makes an entity a page container, and a pane is only
/// the usual one. A demo binary or a test harness can spawn a single
/// full-window node, hang pages off it, and switch between them with no dock in
/// the world at all — which is how a page's own crate stays testable without
/// standing up a layout.
pub fn apply_active_page(
    containers: Query<(&ActivePage, &Children), Or<(Changed<ActivePage>, Changed<Children>)>>,
    mut pages: Query<(&PageId, &mut Node), With<Page>>,
) {
    for (active, children) in &containers {
        for child in children.iter() {
            let Ok((id, mut node)) = pages.get_mut(child) else {
                continue;
            };
            let want = if active.is(id.as_str()) {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != want {
                node.display = want;
            }
        }
    }
}

/// Run condition: true while `id` is the active page of some pane.
///
/// Lets a page's systems go inert when the user is looking at something else,
/// without that page's crate knowing what else exists.
///
/// Deliberately not scoped to a particular pane: a page id is unique within its
/// pane, and a host that puts the same id in two panes has two instances of one
/// thing, which is a naming problem rather than something to disambiguate here.
pub fn page_is_active(id: &'static str) -> impl Fn(Query<&ActivePage>) -> bool + Clone {
    move |panes| panes.iter().any(|active| active.is(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_gets_its_metadata_peers() {
        let mut world = World::new();
        let page = world.spawn(Page).id();

        assert!(world.get::<PageId>(page).is_some());
        assert!(world.get::<PageLabel>(page).is_some());
        assert!(world.get::<PageOrder>(page).is_some());
        assert!(
            world.get::<PageIcon>(page).is_none(),
            "an icon has no sensible default and must stay opt-in"
        );
    }

    #[test]
    fn active_page_matches_by_id() {
        let active = ActivePage::at("timeline");
        assert!(active.is("timeline"));
        assert!(!active.is("spectral"));
        assert!(!ActivePage::none().is("timeline"));
        assert_eq!(active.id(), Some("timeline"));
    }

    #[test]
    fn resolve_falls_back_to_the_first_available_page() {
        // A session naming a page this build dropped must not blank the pane.
        let saved = ActivePage::at("retired");
        assert_eq!(saved.resolve(&["timeline", "spectral"]), Some("timeline"));
    }

    #[test]
    fn resolve_keeps_a_page_that_still_exists() {
        let saved = ActivePage::at("spectral");
        assert_eq!(saved.resolve(&["timeline", "spectral"]), Some("spectral"));
    }

    #[test]
    fn resolve_of_nothing_is_nothing() {
        assert_eq!(ActivePage::at("anything").resolve(&[]), None);
        assert_eq!(ActivePage::none().resolve(&[]), None);
    }

    #[test]
    fn resolve_with_no_active_page_picks_the_first() {
        assert_eq!(ActivePage::none().resolve(&["a", "b"]), Some("a"));
    }
}
