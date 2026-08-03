//! Where a widget's geometry comes from.
//!
//! # The problem this solves
//!
//! Widget sizes were `const fn` tables — `ButtonSize::height()` returning a
//! hardcoded `32.0`, `SliderSize::track_height()` returning `4.0`. A `const
//! fn` cannot read a resource, so the table *had* to restate its numbers,
//! and the restatement is what let them drift: `ButtonSize::Sm` is 28 and
//! [`Sizing::control_sm`] is 28, the same quantity written twice with
//! nothing linking them.
//!
//! [`Metrics`] is the read-only slice of tokens a spawn function consults
//! instead. It is a [`SystemParam`], so a widget declares its geometry
//! dependency the same way it already declares its colour dependency.
//!
//! # What is derived and what is declared
//!
//! Measured across the widget set: **30 of 61 size constants are already
//! members of the spacing scale, and 31 are not** — and the 31 are not
//! uniformly wrong. They split three ways, and the split is the reason this
//! module offers several accessors rather than one:
//!
//! - **Control heights** (28, 32, 44) — [`Sizing`], declared from the grid.
//!   Deliberately off the spacing scale; see [`super::ControlHeight`].
//! - **Scale members** (4, 6, 8, 12, 16, 24, 32) — [`Metrics::gap`] and
//!   friends. These are the ones a base change should move.
//! - **Content measures** (a 720px dialog, a 520px command palette, a 240px
//!   input) — answering to reading comfort and to the content inside them,
//!   not to any grid. [`Metrics::measure`] passes these through and says so.
//!
//! Forcing one rule onto all three would be the same error the hand-tuning
//! made, only automated.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::base::Step;
use super::config::{Sizing, Tokens};
use super::scale::{ControlHeight, Gap, Radius};
use super::{Spacing, Typography};

/// The geometry a widget reads when it lays itself out.
///
/// Pair it with the widget's existing `*Style` bundle, which carries the
/// colour half:
///
/// ```ignore
/// fn spawn_thing(commands: &mut Commands, style: &ThingStyle, metrics: &Metrics) {
///     Node {
///         height: metrics.control(ControlSize::Md).into(),
///         padding: UiRect::horizontal(metrics.gap(Step::new(2)).into()),
///         ..default()
///     };
/// }
/// ```
#[derive(SystemParam)]
pub struct Metrics<'w> {
    /// The generated scale and the declared heights.
    pub tokens: Res<'w, Tokens>,
    /// The named spacing rungs, for call sites that still read them.
    pub spacing: Res<'w, Spacing>,
    /// The type ramp.
    pub typography: Res<'w, Typography>,
}

/// Which of the three declared control heights a widget wants.
///
/// Three rather than a continuum because these are alignment classes: two
/// controls sharing a size sit on the same baseline in a row, and that is
/// the property the enum exists to make checkable.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ControlSize {
    /// Dense rows — menu items, list rows, compact toolbars.
    Sm,
    /// The default: buttons, inputs, selects.
    #[default]
    Md,
    /// Title bars and primary actions.
    Lg,
}

impl Sizing {
    /// The declared height for a control size.
    #[must_use]
    #[inline]
    pub fn get(&self, size: ControlSize) -> ControlHeight {
        match size {
            ControlSize::Sm => self.control_sm,
            ControlSize::Md => self.control_md,
            ControlSize::Lg => self.control_lg,
        }
    }
}

impl Metrics<'_> {
    /// The declared height for a control size.
    #[must_use]
    #[inline]
    pub fn control(&self, size: ControlSize) -> ControlHeight {
        self.tokens.sizing.get(size)
    }

    /// The gap at a step on the generated scale.
    #[must_use]
    #[inline]
    pub fn gap(&self, step: Step) -> Gap {
        self.tokens.scale.at(step)
    }

    /// The radius at a step on the generated scale.
    #[must_use]
    #[inline]
    pub fn radius(&self, step: Step) -> Radius {
        self.tokens.scale.radius_at(step)
    }

    /// The vertical padding that centres text of the given size inside a
    /// control of the given height.
    ///
    /// This is the direction that works — height declared from the grid,
    /// padding *solved*. The inverse (`height = line_height + 2·padding`)
    /// lets content dictate height, which drifts controls off the grid until
    /// they stop aligning with each other.
    ///
    /// Clamped at zero: [`super::ThemeConfig::build`] already rejects a
    /// config where the text cannot fit, so a negative result here would
    /// mean the caller assembled `Sizing` by hand.
    #[must_use]
    pub fn text_inset(&self, size: ControlSize, font_px: f32) -> Gap {
        let line_box = (font_px * super::config::LINE_HEIGHT_RATIO).ceil();
        Gap::px(((self.control(size).get() - line_box) * 0.5).max(0.0))
    }

    /// A length that answers to its content rather than to the grid.
    ///
    /// A dialog is 720px wide because that is a comfortable reading measure;
    /// a command palette is 520px because that is what its rows need. No
    /// scale predicts these, and snapping them to one would be false
    /// precision.
    ///
    /// Call this rather than writing the literal inline, so that a reader
    /// can tell "deliberately off-scale" from "nobody got to it yet" —
    /// which, today, is exactly the distinction that cannot be made by
    /// looking at the code.
    #[must_use]
    #[inline]
    pub fn measure(&self, px: f32) -> Gap {
        Gap::px(px)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Base, Scale, ThemeConfig};

    fn metrics_world() -> App {
        let mut app = App::new();
        app.add_plugins(crate::ThemePlugin);
        app
    }

    fn read<T: Send + 'static>(app: &mut App, f: impl Fn(Metrics) -> T + Send + 'static) -> T {
        let mut state: bevy::ecs::system::SystemState<Metrics> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let m = state.get(app.world()).expect("theme resources present");
        f(m)
    }

    #[test]
    fn the_control_heights_are_the_ones_in_use_today() {
        // `ButtonSize::Sm` is 28 and `text_input::HEIGHT` is 32 — the same
        // two quantities this returns. If these drift apart, the widgets
        // that adopted `Metrics` stop aligning with the ones that have not.
        let mut app = metrics_world();
        let (sm, md, lg) = read(&mut app, |m| {
            (
                m.control(ControlSize::Sm).get(),
                m.control(ControlSize::Md).get(),
                m.control(ControlSize::Lg).get(),
            )
        });
        assert_eq!((sm, md, lg), (28.0, 32.0, 44.0));
    }

    #[test]
    fn a_gap_comes_off_the_generated_scale() {
        let mut app = metrics_world();
        let got = read(&mut app, |m| m.gap(Step::new(2)).get());
        assert_eq!(got, Scale::new(Base::FOUR).at(Step::new(2)).get());
        assert_eq!(got, 8.0);
    }

    #[test]
    fn text_is_centred_in_its_control() {
        // The padding is solved from the height, not the other way round.
        let mut app = metrics_world();
        let inset = read(&mut app, |m| m.text_inset(ControlSize::Md, 14.0).get());
        // 32 tall, a 14px font needs ceil(19.6) = 20 of line box, so 6 above
        // and 6 below.
        assert_eq!(inset, 6.0);
    }

    #[test]
    fn a_control_too_short_for_its_text_insets_by_zero_not_by_a_negative() {
        // `build` rejects this pairing, so reaching it means a caller
        // assembled `Sizing` by hand. A negative inset would lay out as a
        // wildly oversized element rather than as clipped text, which is a
        // much harder symptom to trace back here.
        let mut app = metrics_world();
        let inset = read(&mut app, |m| m.text_inset(ControlSize::Sm, 96.0).get());
        assert_eq!(inset, 0.0);
    }

    #[test]
    fn a_coarser_base_moves_gaps_but_not_declared_heights() {
        // The whole point of the derived/declared split. A base change is a
        // change to the *spacing* grid; control heights answer to hit
        // targets and stay put unless density moves.
        let mut app = App::new();
        app.insert_resource(ThemeConfig {
            base: Base::EIGHT,
            ..default()
        });
        let tokens = ThemeConfig {
            base: Base::EIGHT,
            ..default()
        }
        .build()
        .expect("base 8 comfortable is coherent");
        app.insert_resource(tokens).add_plugins(crate::ThemePlugin);

        let (gap, height) = read(&mut app, |m| {
            (m.gap(Step::new(2)).get(), m.control(ControlSize::Md).get())
        });
        assert_eq!(gap, 16.0, "the scale doubled with the base");
        // The height did *not* double, and that is the point rather than an
        // accident: a hit target is a fact about fingers, so it does not
        // move when the spacing grid gets coarser. It is exactly why the
        // base-8 multipliers in `Density` are a separate row of the table
        // instead of the base-4 ones reused.
        assert_eq!(height, 32.0, "a coarser grid must not inflate a control");
    }
    #[test]
    fn every_substituted_const_still_returns_its_old_value() {
        // This PR promised nothing moves on screen. Each of these was a
        // literal before; each is now a scale lookup. The two must agree at
        // the default base, or the change was not invisible after all.
        let t = crate::ThemeConfig::default().build().unwrap();
        let at = |n: i8| t.scale.at(Step::new(n)).get();

        // toast: padding, corner radius, spacing, margin, progress height
        assert_eq!(at(4), 16.0);
        assert_eq!(at(2), 8.0);
        assert_eq!(at(-2), 2.0);
        // checkbox: box, corner
        assert_eq!(at(4), 16.0);
        assert_eq!(at(0), 4.0);
        // progress height, tabs trigger padding, command section padding
        assert_eq!(at(2), 8.0);
        // toggle_group item padding, tooltip padding-x, dialog card radius
        assert_eq!(at(3), 12.0);
        // tooltip corner radius + padding-y
        assert_eq!(at(1), 6.0);
        // toggle_group item height
        assert_eq!(t.sizing.get(ControlSize::Sm).get(), 28.0);
    }

    #[test]
    fn snapping_a_stray_puts_its_derived_values_on_the_scale_too() {
        // The argument for moving these was not "36 is not a round number".
        // It was that each stray had something *derived* from it, and the
        // derived value was off the scale as well — so the old number was
        // less coherent, not merely different.
        let t = crate::ThemeConfig::default().build().unwrap();
        let on_scale = |v: f32| (-3i8..9).any(|n| t.scale.at(Step::new(n)).get() == v);

        // A command-palette row contains a 16px icon. At the old height of 36
        // the vertical gutter was 10, which is on no scale; at `control_md`
        // it is 8, which is step 2.
        let item = t.sizing.get(ControlSize::Md).get();
        assert_eq!(item, 32.0);
        assert!(
            on_scale((item - 16.0) / 2.0),
            "the icon's gutter is on-scale"
        );
        assert!(!on_scale((36.0 - 16.0) / 2.0), "at 36 it was not");

        // A tab trigger is `strip - 2 * inset`. At the old strip height of 26
        // that was 22; at `control_sm` it is 24, which is step 5.
        let strip = t.sizing.get(ControlSize::Sm).get();
        assert_eq!(strip, 28.0);
        assert!(
            on_scale(strip - 2.0 * 2.0),
            "the trigger's height is on-scale"
        );
        assert!(!on_scale(26.0 - 2.0 * 2.0), "at 26 it was not");

        // The dialog's title bar sits inside a 12px card radius. At 16 of
        // padding the offset between the two is 4 — which is exactly what
        // `concentric_inner` describes. At 18 it was 6, which is on the scale
        // by coincidence but is not the concentric answer.
        let radius = t.scale.radius_at(Step::new(3));
        let padding = t.scale.at(Step::new(4));
        assert_eq!(radius.get(), 12.0);
        assert_eq!(padding.get(), 16.0);
        assert_eq!(
            radius.concentric_inner(t.scale.at(Step::BASE)).get(),
            8.0,
            "a child inset by 4 from a 12px corner nests at 8"
        );
    }
}
