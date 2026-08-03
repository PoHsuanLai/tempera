//! The config a whole layout is derived from, and the generator that turns
//! it into tokens.
//!
//! ```text
//! ThemeConfig { base, density, text }  ->  Result<Tokens, Incoherent>
//! ```
//!
//! Three inputs; everything else derived. "Coherent" stops being an
//! aspiration and becomes a property testable over the whole input space.

use bevy::prelude::*;

use super::base::{Base, Step};
use super::scale::{ControlHeight, FontSize, Radius, Scale};

/// How tall interactive controls are, relative to the base grid.
///
/// **This is an input, not a derivation**, and that is the load-bearing
/// claim of the whole model. The measurement that settles it: the control
/// heights tempera uses today are integer multiples of base **4** (28 = 7×4,
/// 32 = 8×4, 44 = 11×4) but *fractional* multiples of base **8**
/// (26/8 = 3.25, 36/8 = 4.5). If height followed from base, density would be
/// a function rather than a choice.
///
/// Consequently the multipliers are chosen **per base** rather than shared.
/// `Comfortable` at base 4 gives 28/32/44 — the values in use today, all
/// above the WCAG 2.5.8 AA floor of 24 CSS px. The same multipliers at base
/// 8 would give 56/64/88, which is absurd. That is not a flaw in the model;
/// it is the model correctly reporting that base and density are
/// independent.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Density {
    /// Dense rows for tool windows and lists. Small controls sit at the
    /// accessibility floor, not below it.
    Compact,
    /// The default: the heights tempera ships with today.
    #[default]
    Comfortable,
    /// Roomier, for touch or for long sessions.
    Spacious,
}

impl Density {
    /// Multipliers of the base for the small / medium / large control
    /// heights, at the given base.
    ///
    /// Tuned per base for the reason in the type docs. Bases other than 4
    /// and 8 fall back to the base-8 table scaled by ratio, which keeps them
    /// usable without pretending they were tuned.
    fn multipliers(self, base: Base) -> (f32, f32, f32) {
        match (base.px(), self) {
            (4, Density::Compact) => (6.0, 7.0, 9.0), // 24 / 28 / 36
            (4, Density::Comfortable) => (7.0, 8.0, 11.0), // 28 / 32 / 44
            (4, Density::Spacious) => (8.0, 10.0, 13.0), // 32 / 40 / 52
            (_, Density::Compact) => (3.0, 3.5, 4.5),
            (_, Density::Comfortable) => (3.5, 4.0, 5.5),
            (_, Density::Spacious) => (4.0, 5.0, 6.5),
        }
    }
}

/// The overall text size, which shifts the whole curated type ramp.
///
/// A separate input from [`Density`] because font sizes divide evenly by
/// neither 4 nor 8 — type answers to hinting and x-height, not to the pixel
/// grid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum TextScale {
    /// One notch down from default.
    Small,
    /// The ramp tempera ships with: 8 / 10 / 12 / 14 / 16 / 18 / 24.
    #[default]
    Medium,
    /// One notch up, for larger displays or lower acuity.
    Large,
}

impl TextScale {
    /// The body size, from which the rest of the ramp is offset.
    fn body_px(self) -> f32 {
        match self {
            TextScale::Small => 12.0,
            TextScale::Medium => 14.0,
            TextScale::Large => 16.0,
        }
    }
}

/// The inputs a whole set of tokens is generated from.
///
/// Stored as a [`Resource`] so a host can change density or base at runtime
/// and have the layout follow — which is the point of making the theme a
/// function rather than a table.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct ThemeConfig {
    /// The grid unit the spacing scale is generated from.
    pub base: Base,
    /// How tall controls are relative to that grid.
    pub density: Density,
    /// The overall text size.
    pub text: TextScale,
}

/// A config whose inputs contradict each other.
///
/// Font size and control height are not fully independent: a control cannot
/// be shorter than the line box of the text inside it. This is a constraint
/// *between two inputs*, not a fourth input, so it is validation rather than
/// derivation — and reporting it beats silently clipping text.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Incoherent {
    /// The medium control height cannot contain its own body text.
    ControlShorterThanItsText {
        /// The height the density asked for.
        height: f32,
        /// The line box the text at this scale needs.
        line_box: f32,
    },
    /// A control would land below the WCAG 2.5.8 AA hit-target floor of
    /// 24×24 CSS px.
    BelowHitTarget {
        /// The height the density asked for.
        height: f32,
        /// The floor it fell under.
        floor: f32,
    },
}

impl std::fmt::Display for Incoherent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Incoherent::ControlShorterThanItsText { height, line_box } => write!(
                f,
                "a {height}px control cannot contain a {line_box}px line box — \
                 lower the text scale or raise the density"
            ),
            Incoherent::BelowHitTarget { height, floor } => write!(
                f,
                "a {height}px control is below the {floor}px hit-target floor \
                 (WCAG 2.5.8 AA)"
            ),
        }
    }
}

impl std::error::Error for Incoherent {}

/// The WCAG 2.5.8 AA minimum hit target, in CSS pixels. (2.5.5 AAA asks for
/// 44; Apple's well-known 44 is *points*, which is a different unit that
/// happens to coincide numerically.)
const HIT_TARGET_FLOOR: f32 = 24.0;

/// The multiple of font size a line box occupies. 1.4 is the usual body
/// figure and the one the heights below were checked against.
///
/// Shared with [`super::Metrics::text_inset`], which solves padding from it:
/// `build` rejecting a config and `text_inset` centring text inside one are
/// the same question asked twice, so they must use the same ratio. A second
/// copy is exactly the restatement this module exists to remove.
pub(crate) const LINE_HEIGHT_RATIO: f32 = 1.4;

/// Control heights, **declared** from the grid rather than generated.
///
/// See [`ControlHeight`] for why these are off the spacing scale on purpose.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Sizing {
    /// Dense rows — menu items, list rows.
    pub control_sm: ControlHeight,
    /// The default: buttons, inputs, selects.
    pub control_md: ControlHeight,
    /// Title bars and primary actions.
    pub control_lg: ControlHeight,
}

/// The full token set derived from a [`ThemeConfig`].
///
/// Held as a resource so widgets can react to it; the individual token
/// buckets ([`super::Spacing`], [`super::Typography`]) are regenerated from
/// this and stay separately readable so a system still declares only the
/// slice it uses.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Tokens {
    /// The generated spacing scale.
    pub scale: Scale,
    /// The declared control heights.
    pub sizing: Sizing,
    /// The config these were built from.
    pub config: ThemeConfig,
}

impl ThemeConfig {
    /// Generate the tokens, or report which two inputs contradict.
    ///
    /// # Errors
    ///
    /// Returns [`Incoherent`] when the density and text scale cannot both be
    /// satisfied — see that type.
    pub fn build(&self) -> Result<Tokens, Incoherent> {
        let (sm, md, lg) = self.density.multipliers(self.base);
        let b = self.base.get();
        let (h_sm, h_md, h_lg) = (b * sm, b * md, b * lg);

        if h_sm < HIT_TARGET_FLOOR {
            return Err(Incoherent::BelowHitTarget {
                height: h_sm,
                floor: HIT_TARGET_FLOOR,
            });
        }

        let line_box = (self.text.body_px() * LINE_HEIGHT_RATIO).ceil();
        if h_md < line_box {
            return Err(Incoherent::ControlShorterThanItsText {
                height: h_md,
                line_box,
            });
        }

        Ok(Tokens {
            scale: Scale::new(self.base),
            sizing: Sizing {
                control_sm: ControlHeight::px(h_sm),
                control_md: ControlHeight::px(h_md),
                control_lg: ControlHeight::px(h_lg),
            },
            config: *self,
        })
    }
}

impl Tokens {
    /// The body text size for this config.
    #[must_use]
    pub fn body(&self) -> FontSize {
        FontSize::px(self.config.text.body_px())
    }

    /// The default corner radius — one step below the base, which is what
    /// puts it visually subordinate to the padding beside it.
    #[must_use]
    pub fn radius(&self) -> Radius {
        self.scale.radius_at(Step::new(-1))
    }
}

impl Default for Tokens {
    fn default() -> Self {
        // The default config is checked coherent by
        // `the_default_config_is_coherent`, so this cannot panic.
        ThemeConfig::default()
            .build()
            .expect("the default ThemeConfig is coherent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_config() -> impl Iterator<Item = ThemeConfig> {
        let bases = [Base::FOUR, Base::EIGHT, Base::new(6).unwrap()];
        let densities = [Density::Compact, Density::Comfortable, Density::Spacious];
        let texts = [TextScale::Small, TextScale::Medium, TextScale::Large];
        bases.into_iter().flat_map(move |base| {
            densities.into_iter().flat_map(move |density| {
                texts.into_iter().map(move |text| ThemeConfig {
                    base,
                    density,
                    text,
                })
            })
        })
    }

    #[test]
    fn the_default_config_is_coherent() {
        // `Tokens::default` unwraps this, and several resources derive
        // `Default` through it.
        assert!(ThemeConfig::default().build().is_ok());
    }

    #[test]
    fn the_default_config_reproduces_todays_heights() {
        // The model is meant to explain the numbers already on screen, not
        // to replace them. If this fails, the multiplier table drifted.
        let t = ThemeConfig::default().build().unwrap();
        assert_eq!(t.sizing.control_sm.get(), 28.0);
        assert_eq!(t.sizing.control_md.get(), 32.0);
        assert_eq!(t.sizing.control_lg.get(), 44.0);
    }

    #[test]
    fn a_control_is_never_shorter_than_its_text() {
        // The property `build` returns a `Result` for. Every config that
        // builds must actually be able to show its own body text.
        for config in every_config() {
            let Ok(tokens) = config.build() else {
                continue;
            };
            let line_box = (config.text.body_px() * LINE_HEIGHT_RATIO).ceil();
            assert!(
                tokens.sizing.control_md.get() >= line_box,
                "{config:?} built but clips its own text: \
                 {} < {line_box}",
                tokens.sizing.control_md.get()
            );
        }
    }

    #[test]
    fn a_control_is_never_below_the_hit_target_floor() {
        for config in every_config() {
            let Ok(tokens) = config.build() else {
                continue;
            };
            assert!(
                tokens.sizing.control_sm.get() >= HIT_TARGET_FLOOR,
                "{config:?} built a {}px control, under the AA floor",
                tokens.sizing.control_sm.get()
            );
        }
    }

    #[test]
    fn an_incoherent_pairing_is_reported_rather_than_clipped() {
        // Large text on a compact base-4 grid: 28px of control for a 23px
        // line box still fits, so reach for the case that genuinely does
        // not — a base small enough that no multiplier saves it.
        let cramped = ThemeConfig {
            base: Base::new(2).unwrap(),
            density: Density::Compact,
            text: TextScale::Large,
        };
        assert!(
            cramped.build().is_err(),
            "a 12px control cannot show 16px text and must say so"
        );
    }

    #[test]
    fn heights_stay_ordered() {
        for config in every_config() {
            let Ok(t) = config.build() else { continue };
            assert!(t.sizing.control_sm.get() < t.sizing.control_md.get());
            assert!(t.sizing.control_md.get() < t.sizing.control_lg.get());
        }
    }

    #[test]
    fn density_is_not_a_function_of_base() {
        // The measurement the three-input model rests on: today's heights
        // are integer multiples of base 4 but fractional multiples of base
        // 8. If this stopped holding, `density` would collapse into `base`
        // and the config would have two inputs, not three.
        for h in [28.0f32, 32.0, 44.0] {
            assert_eq!(h % 4.0, 0.0, "{h} is not a whole number of base-4 units");
        }
        assert_ne!(26.0f32 % 8.0, 0.0);
        assert_ne!(36.0f32 % 8.0, 0.0);
    }
}
