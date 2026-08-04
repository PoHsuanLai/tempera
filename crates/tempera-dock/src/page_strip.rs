//! A row of chips for choosing a pane's page.
//!
//! [`page`](crate::page) says which page is showing and hides the rest; it
//! deliberately draws no chooser. This is one — a segmented control, styled as
//! a single pill, one chip per [`Page`] of the pane it charts.
//!
//! ```
//! use bevy::prelude::*;
//! use tempera_dock::{PageStrip, PaneRegistry};
//!
//! fn add_strip(mut commands: Commands, panes: Res<PaneRegistry>) {
//!     let pane = panes.get("center").expect("declared in the layout");
//!     commands.spawn((PageStrip(pane), ChildOf(pane)));
//! }
//! ```
//!
//! # The strip is a view
//!
//! [`ActivePage`] on the pane is the state and its only owner. A chip reads it
//! to paint and writes it on click, and holds no selection of its own — delete
//! the strip and the pane still works, because a keybind, a command or a
//! dropdown sets the same field.
//!
//! That is why nothing here caches "am I selected". A chip that stored a `bool`
//! would go stale the moment something *other* than the strip changed the page,
//! and the strip would highlight the wrong chip with nothing pointing at the
//! cause. [`repaint_chips`] recomputes from [`ActivePage`] every time, which is
//! one comparison per chip.
//!
//! # The strip names its pane
//!
//! [`PageStrip`] carries the pane entity rather than finding it by walking up
//! the hierarchy, so a host may put the strip anywhere — inside the pane, in a
//! title bar, in a status line at the other end of the window. A walk-up would
//! forbid all but the first, and costs an ancestor search on every repaint.
//!
//! # Chips are reconciled, not spawned once
//!
//! Pages appear when their crate is compiled in, which can be after the strip
//! exists. The implementation this replaces guarded with
//! `if !existing.is_empty() { return; }` and so showed a page registered at
//! startup and nothing after — a plugin ordering change was enough to lose a
//! chip. [`reconcile_chips`] diffs instead, in the same spirit as the dock's own
//! rebuild: declare the pages, resolve the chips.

use bevy::prelude::*;
use bevy::ui::Checked;
use bevy_resvg::prelude::{SvgColor, UiSvg};
use tempera_theme::ColorPalette;

use crate::page::{ActivePage, Page, PageIcon, PageId, PageLabel, PageOrder};

/// A chooser for the pages of `0`, the pane it charts.
///
/// Spawn one anywhere; it need not be a child of the pane. Chips are its
/// children and are reconciled from the pane's [`Page`] entities.
#[derive(Component, Debug, Clone, Copy)]
#[require(Node, PageStripStyle)]
pub struct PageStrip(pub Entity);

/// Which page a chip stands for, and which pane that page belongs to.
///
/// The pane is repeated here rather than read from the parent [`PageStrip`]
/// because the click observer needs it, and an observer that walks to its
/// parent to find a value the spawn already knew is a lookup for nothing.
#[derive(Component, Debug, Clone)]
pub struct PageChip {
    /// The page this chip selects.
    pub page: PageId,
    /// The pane whose [`ActivePage`] the click writes.
    pub pane: Entity,
}

/// Per-strip appearance. Every field resolves against the palette when unset,
/// so a host that inserts nothing still gets themed chips.
#[derive(Component, Debug, Clone, Copy)]
pub struct PageStripStyle {
    /// Chip height, in logical pixels.
    pub height: f32,
    /// Glyph size inside a chip.
    pub icon_size: f32,
    /// Radius of the strip's two outer ends.
    pub end_radius: f32,
    /// Fill of the chosen chip. Falls back to `palette.primary`.
    pub selected: Option<Color>,
    /// Fill of the others. Falls back to `palette.muted`.
    pub resting: Option<Color>,
}

impl Default for PageStripStyle {
    fn default() -> Self {
        Self {
            height: 24.0,
            icon_size: 14.0,
            end_radius: 6.0,
            selected: None,
            resting: None,
        }
    }
}

/// Colours a chip can take, resolved once per repaint.
struct ChipColors {
    selected_bg: Color,
    resting_bg: Color,
    hovered_bg: Color,
    selected_fg: Color,
    resting_fg: Color,
}

impl ChipColors {
    fn resolve(style: &PageStripStyle, palette: &ColorPalette) -> Self {
        let selected_bg = style.selected.unwrap_or(palette.primary);
        let resting_bg = style.resting.unwrap_or(palette.muted);
        Self {
            selected_bg,
            resting_bg,
            // Lift toward the foreground rather than toward white, so the
            // hover reads the same in a light theme as in a dark one.
            hovered_bg: ColorPalette::step(resting_bg, palette.background, HOVER_LIFT),
            selected_fg: palette.primary_foreground,
            resting_fg: palette.muted_foreground,
        }
    }
}

/// How far a hovered chip steps away from its surface.
const HOVER_LIFT: f32 = 0.6;

/// Spawn a chip per page, drop chips whose page is gone, and keep them ordered.
///
/// One system for all three because they share the sorted page list, and
/// because splitting them would let a frame land between "spawned" and
/// "ordered" — a chip appearing at the wrong end and jumping is visible.
///
/// Gating on `Changed<Children>` of the *pane* is not enough: a page's
/// [`PageOrder`] can change without the child set moving. The diff runs
/// whenever a page is added, removed or reordered.
pub(crate) fn reconcile_chips(
    mut commands: Commands,
    strips: Query<(Entity, &PageStrip, &PageStripStyle, Option<&Children>)>,
    panes: Query<&Children>,
    pages: Query<(&PageId, Option<&PageLabel>, Option<&PageIcon>, &PageOrder), With<Page>>,
    chips: Query<&PageChip>,
    added: Query<(), Added<Page>>,
    moved: Query<(), Changed<PageOrder>>,
    mut removed: RemovedComponents<Page>,
) {
    // `RemovedComponents` must be drained whether or not it is used, or the
    // events pile up until another trigger happens to fire.
    let any_removed = removed.read().count() > 0;
    if added.is_empty() && moved.is_empty() && !any_removed {
        return;
    }

    for (strip_entity, strip, style, strip_children) in &strips {
        let mut wanted: Vec<Candidate> = panes
            .get(strip.0)
            .map(|kids| {
                kids.iter()
                    .filter_map(|kid| pages.get(kid).ok())
                    .map(|(id, label, icon, order)| Candidate {
                        order: order.0,
                        id,
                        label,
                        icon,
                    })
                    .collect()
            })
            .unwrap_or_default();
        // Ties break on id so the order is total — two pages at the same
        // `PageOrder` must not swap places between frames.
        wanted.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(b.id)));

        let existing: Vec<Entity> = strip_children
            .map(|kids| kids.iter().filter(|kid| chips.contains(*kid)).collect())
            .unwrap_or_default();

        // Rebuild only when the set or the order actually differs. Without
        // this the strip re-spawns every chip on any page change anywhere,
        // which drops hover state and restarts any transition mid-flight.
        let same = existing.len() == wanted.len()
            && existing
                .iter()
                .zip(&wanted)
                .all(|(chip, want)| chips.get(*chip).is_ok_and(|c| &c.page == want.id));
        if same {
            continue;
        }

        for chip in existing {
            commands.entity(chip).despawn();
        }

        let last = wanted.len().saturating_sub(1);
        for (index, want) in wanted.iter().enumerate() {
            spawn_chip(
                &mut commands,
                Placement {
                    strip: strip_entity,
                    pane: strip.0,
                    index,
                    last,
                },
                style,
                want,
            );
        }
    }
}

/// One page that ought to have a chip, with the metadata the chip needs.
struct Candidate<'a> {
    order: i32,
    id: &'a PageId,
    label: Option<&'a PageLabel>,
    icon: Option<&'a PageIcon>,
}

/// Where a chip sits: whose child it is, which pane it writes, and its
/// position in the row — which is what decides the end-cap rounding.
struct Placement {
    strip: Entity,
    pane: Entity,
    index: usize,
    last: usize,
}

/// One chip. Square inner corners, rounded outer ends, so the row reads as a
/// single pill rather than a line of separate buttons.
fn spawn_chip(commands: &mut Commands, at: Placement, style: &PageStripStyle, page: &Candidate) {
    let zero = Val::Px(0.0);
    let end = Val::Px(style.end_radius);
    // A lone chip is both first and last, and rounds all four.
    let border_radius = BorderRadius {
        top_left: if at.index == 0 { end } else { zero },
        bottom_left: if at.index == 0 { end } else { zero },
        top_right: if at.index == at.last { end } else { zero },
        bottom_right: if at.index == at.last { end } else { zero },
    };

    // The label if there is one, else the id — an inspector row reading
    // `page_chip::` and nothing else names nothing.
    let name = page
        .label
        .map_or_else(|| page.id.as_str().to_owned(), |label| label.0.to_owned());

    let chip = commands
        .spawn((
            PageChip {
                page: page.id.clone(),
                pane: at.pane,
            },
            Node {
                height: Val::Px(style.height),
                // Equal shares of the strip regardless of glyph size:
                // `flex_basis: 0` is what makes the grow split exact.
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Interaction::default(),
            Name::new(format!("tempera::page_chip::{name}")),
            ChildOf(at.strip),
        ))
        .observe(on_chip_click)
        .id();

    if let Some(icon) = page.icon {
        commands.spawn((
            UiSvg(icon.0.clone()),
            Node {
                width: Val::Px(style.icon_size),
                height: Val::Px(style.icon_size),
                ..default()
            },
            // The chip is the click target; the glyph must not intercept.
            bevy::picking::Pickable::IGNORE,
            ChildOf(chip),
        ));
    }
}

/// Write the clicked chip's page onto its pane.
///
/// The only mutation in this module. A chip stores no selection — it names a
/// page and a pane, and [`ActivePage`] decides what that means.
fn on_chip_click(
    mut click: On<Pointer<Click>>,
    chips: Query<&PageChip>,
    mut panes: Query<&mut ActivePage>,
) {
    let Ok(chip) = chips.get(click.entity) else {
        return;
    };
    if let Ok(mut active) = panes.get_mut(chip.pane)
        && !active.is(chip.page.as_str())
    {
        active.set(chip.page.clone());
    }
    click.propagate(false);
}

/// Paint each chip from its pane's [`ActivePage`] and its own hover.
///
/// Unfiltered rather than gated on `Changed<ActivePage>`, for the reason
/// [`crate::page`]'s neighbours give: a palette swap makes every chip stale at
/// once and no per-entity filter can say so. The write compares first, so a
/// settled strip costs one comparison per chip and marks nothing dirty.
///
/// [`Checked`] is declared alongside the fill so a host can style or query the
/// selection without re-deriving it, and so assistive tooling has something to
/// read. The colour is still resolved here — the marker is state, not paint.
pub(crate) fn repaint_chips(
    palette: Option<Res<ColorPalette>>,
    strips: Query<(&PageStripStyle, Option<&Children>), With<PageStrip>>,
    panes: Query<&ActivePage>,
    mut chips: Query<(
        Entity,
        &PageChip,
        &Interaction,
        Has<Checked>,
        &mut BackgroundColor,
    )>,
    icons: Query<(Entity, Option<&SvgColor>), With<UiSvg>>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    // No palette means no theme plugin — a headless host or a test standing up
    // the dock alone. Chips keep their geometry and go unpainted, the same
    // choice the divider's hover tint makes.
    let Some(palette) = palette else { return };

    for (style, strip_children) in &strips {
        let Some(strip_children) = strip_children else {
            continue;
        };
        let colors = ChipColors::resolve(style, &palette);

        for child in strip_children.iter() {
            let Ok((entity, chip, interaction, checked, mut bg)) = chips.get_mut(child) else {
                continue;
            };
            let selected = panes
                .get(chip.pane)
                .is_ok_and(|active| active.is(chip.page.as_str()));
            let hovered = !matches!(interaction, Interaction::None);

            let want_bg = match (selected, hovered) {
                (true, _) => colors.selected_bg,
                (false, true) => colors.hovered_bg,
                (false, false) => colors.resting_bg,
            };
            if bg.0 != want_bg {
                bg.0 = want_bg;
            }

            if selected != checked {
                if selected {
                    commands.entity(entity).insert(Checked);
                } else {
                    commands.entity(entity).remove::<Checked>();
                }
            }

            let want_fg = if selected {
                colors.selected_fg
            } else {
                colors.resting_fg
            };
            // `SvgColor`, never `ImageNode`: `bevy_resvg` owns that component
            // and its insert query skips entities that already have one, so a
            // write here would make the glyph vanish rather than mis-tint.
            if let Ok(kids) = children.get(entity) {
                for kid in kids.iter() {
                    if let Ok((icon, current)) = icons.get(kid)
                        && current.is_none_or(|c| c.0 != want_fg)
                    {
                        commands.entity(icon).insert(SvgColor(want_fg));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::Page;

    /// A pane with `pages` hung off it, and a strip charting it.
    fn app_with(pages: &[(&str, i32)]) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<ColorPalette>()
            .add_systems(Update, (reconcile_chips, repaint_chips).chain());

        let pane = app.world_mut().spawn(ActivePage::none()).id();
        for (id, order) in pages {
            app.world_mut()
                .spawn((Page, PageId::from(*id), PageOrder(*order), ChildOf(pane)));
        }
        let strip = app.world_mut().spawn((PageStrip(pane), ChildOf(pane))).id();
        app.update();
        (app, pane, strip)
    }

    fn chip_ids(app: &mut App, strip: Entity) -> Vec<String> {
        let kids: Vec<Entity> = app
            .world()
            .get::<Children>(strip)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        kids.iter()
            .filter_map(|k| app.world().get::<PageChip>(*k))
            .map(|c| c.page.0.clone())
            .collect()
    }

    #[test]
    fn a_chip_appears_for_every_page() {
        let (mut app, _, strip) = app_with(&[("timeline", 10), ("spectral", 20)]);
        assert_eq!(chip_ids(&mut app, strip), ["timeline", "spectral"]);
    }

    #[test]
    fn chips_follow_page_order_not_spawn_order() {
        let (mut app, _, strip) = app_with(&[("last", 30), ("first", 10), ("middle", 20)]);
        assert_eq!(chip_ids(&mut app, strip), ["first", "middle", "last"]);
    }

    #[test]
    fn a_page_registered_after_the_strip_still_gets_a_chip() {
        // The failure this widget exists to prevent: the implementation it
        // replaces returned early if any chip existed, so a mode whose crate
        // registered late was permanently unreachable.
        let (mut app, pane, strip) = app_with(&[("timeline", 10)]);
        assert_eq!(chip_ids(&mut app, strip).len(), 1);

        app.world_mut()
            .spawn((Page, PageId::from("spectral"), PageOrder(20), ChildOf(pane)));
        app.update();

        assert_eq!(chip_ids(&mut app, strip), ["timeline", "spectral"]);
    }

    #[test]
    fn a_removed_page_takes_its_chip_with_it() {
        let (mut app, pane, strip) = app_with(&[("timeline", 10), ("doomed", 20)]);

        let doomed = app
            .world_mut()
            .query_filtered::<(Entity, &PageId), With<Page>>()
            .iter(app.world())
            .find(|(_, id)| id.as_str() == "doomed")
            .map(|(e, _)| e)
            .expect("spawned");
        app.world_mut().entity_mut(doomed).despawn();
        app.update();

        assert_eq!(chip_ids(&mut app, strip), ["timeline"]);
        let _ = pane;
    }

    #[test]
    fn only_the_outer_corners_are_rounded() {
        // What makes the row read as one pill. Every inner corner square,
        // and the two ends rounded — an off-by-one here is the difference
        // between a segmented control and a row of loose buttons.
        let (app, _, strip) = app_with(&[("a", 10), ("b", 20), ("c", 30)]);
        let kids: Vec<Entity> = app.world().get::<Children>(strip).unwrap().iter().collect();
        let radii: Vec<BorderRadius> = kids
            .iter()
            .filter_map(|k| app.world().get::<Node>(*k))
            .map(|n| n.border_radius)
            .collect();

        let zero = Val::Px(0.0);
        assert_ne!(radii[0].top_left, zero, "first chip rounds its left");
        assert_eq!(radii[0].top_right, zero, "and squares its right");
        assert_eq!(radii[1].top_left, zero, "a middle chip is square");
        assert_eq!(radii[1].top_right, zero);
        assert_eq!(radii[2].top_left, zero, "last chip squares its left");
        assert_ne!(radii[2].top_right, zero, "and rounds its right");
    }

    #[test]
    fn a_lone_chip_rounds_both_ends() {
        // `index == 0 && index == last` — the case a first/last pair of
        // `if`s gets right and a `match` on position typically does not.
        let (app, _, strip) = app_with(&[("only", 10)]);
        let kid = app.world().get::<Children>(strip).unwrap()[0];
        let radius = app.world().get::<Node>(kid).unwrap().border_radius;

        assert_ne!(radius.top_left, Val::Px(0.0));
        assert_ne!(radius.top_right, Val::Px(0.0));
    }

    #[test]
    fn clicking_a_chip_selects_its_page() {
        let (mut app, pane, strip) = app_with(&[("timeline", 10), ("spectral", 20)]);
        let kids: Vec<Entity> = app.world().get::<Children>(strip).unwrap().iter().collect();

        app.world_mut().trigger(click_at(kids[1]));
        app.update();

        assert_eq!(
            app.world().get::<ActivePage>(pane).unwrap().id(),
            Some("spectral")
        );
    }

    #[test]
    fn the_strip_reads_a_selection_it_did_not_make() {
        // The property that makes this a view: something else — a keybind, a
        // command, a restored session — writes `ActivePage`, and the strip
        // must follow. A chip caching its own selection passes every
        // click-driven test and fails exactly this one.
        let (mut app, pane, strip) = app_with(&[("timeline", 10), ("spectral", 20)]);

        app.world_mut()
            .entity_mut(pane)
            .insert(ActivePage::at("spectral"));
        app.update();

        let kids: Vec<Entity> = app.world().get::<Children>(strip).unwrap().iter().collect();
        assert!(
            app.world().get::<Checked>(kids[1]).is_some(),
            "the chip for the active page must read as chosen"
        );
        assert!(
            app.world().get::<Checked>(kids[0]).is_none(),
            "and no other chip may"
        );
    }

    #[test]
    fn the_chosen_chip_is_painted_apart_from_the_rest() {
        let (mut app, pane, strip) = app_with(&[("timeline", 10), ("spectral", 20)]);
        app.world_mut()
            .entity_mut(pane)
            .insert(ActivePage::at("timeline"));
        app.update();

        let kids: Vec<Entity> = app.world().get::<Children>(strip).unwrap().iter().collect();
        let chosen = app.world().get::<BackgroundColor>(kids[0]).unwrap().0;
        let other = app.world().get::<BackgroundColor>(kids[1]).unwrap().0;
        assert_ne!(chosen, other);
    }

    #[test]
    fn a_strip_whose_pages_did_not_change_keeps_its_chips() {
        // Re-spawning chips drops hover and restarts any transition, so a
        // reconcile that concerns some *other* pane must leave this strip
        // alone.
        //
        // An idle `app.update()` would not test this: the trigger gate at the
        // top of `reconcile_chips` returns before the per-strip diff, so the
        // diff itself would go unexercised and a broken one would still pass.
        // Adding a page elsewhere fires `Added<Page>` globally, which gets
        // past the gate and puts the diff on the hook.
        let (mut app, _, strip) = app_with(&[("a", 10), ("b", 20)]);
        let before: Vec<Entity> = app.world().get::<Children>(strip).unwrap().iter().collect();

        let elsewhere = app.world_mut().spawn(ActivePage::none()).id();
        app.world_mut().spawn((
            Page,
            PageId::from("other"),
            PageOrder(10),
            ChildOf(elsewhere),
        ));
        app.update();

        let after: Vec<Entity> = app.world().get::<Children>(strip).unwrap().iter().collect();
        assert_eq!(before, after, "chips must not churn");
    }

    #[test]
    fn a_strip_charting_a_pane_with_no_pages_is_empty_not_broken() {
        let (mut app, _, strip) = app_with(&[]);
        assert!(chip_ids(&mut app, strip).is_empty());
    }

    /// A primary-button click aimed at `entity`.
    ///
    /// Hand-built because `Pointer`'s `propagate` field is crate-private, so
    /// `Pointer::new` is the only way in from outside `bevy_picking`.
    fn click_at(entity: Entity) -> Pointer<Click> {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::backend::HitData;
        use bevy::picking::pointer::{Location, PointerButton, PointerId};

        Pointer::new(
            PointerId::Mouse,
            Location {
                // The colour-target-less variant: this crate has no
                // `bevy_render`, and a click needs no surface to land on.
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
            Click {
                button: PointerButton::Primary,
                hit: HitData::new(entity, 0.0, None, None),
                duration: core::time::Duration::from_millis(1),
                count: 1,
            },
            entity,
        )
    }
}
