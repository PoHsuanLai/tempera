//! Design tokens for `bevy_ui` — a generated spacing scale, a palette, and a
//! type ramp.
//!
//! # This crate does not know what a button is
//!
//! Nothing here names a widget type. The only thing borrowed from the UI
//! layer is [`bevy::ui::Val`], for the `From` impls that let a token be
//! assigned straight into a `Node` field. That is what makes the tokens
//! separately depend-able: a crate that needs to know what colour the surface
//! is should not have to compile a text-input widget to find out.
//!
//! `tempera-widgets` re-exports this crate as `tempera::theme`, so a consumer
//! that wants both can keep writing one path.
//!
//! # A function, not a table
//!
//! [`ThemeConfig`] is three inputs — a [`Base`], a [`Density`], a
//! [`TextScale`] — and [`ThemeConfig::build`] derives the rest:
//!
//! ```ignore
//! ThemeConfig { base, density, text } -> Result<Tokens, Incoherent>
//! ```
//!
//! [`Spacing`] is generated from the base by a two-strand modular scale (see
//! [`Scale`]); [`Sizing`] is *declared*, because control heights answer to hit
//! targets rather than to perceptual spacing; [`Typography`] is *curated*,
//! because type must survive hinting and x-height, which no ratio models.
//! Each group says which it is in its own docs, so a reader is not left
//! hunting for a rule that was never there.
//!
//! # Buckets, so a system reads only what it uses
//!
//! Each token group is its own [`Resource`]. A typography system doesn't need
//! to fight the colour palette for scheduling slots, and a widget that reads
//! several at once bundles them into a `SystemParam` — see [`Metrics`], which
//! is the geometry half of that pattern.
//!
//! ```ignore
//! // a widget system reads only what it needs:
//! fn paint_buttons(palette: Res<ColorPalette>, spacing: Res<Spacing>, ...) {}
//! ```
//!
//! ## Defaults
//!
//! [`ThemePlugin`] installs dark-mode defaults (shadcn zinc). Override
//! any sub-resource at startup to recolor / resize / re-font that
//! aspect of every widget:
//!
//! ```ignore
//! app.add_plugins(ThemePlugin)
//!    .insert_resource(ColorPalette::light());
//! ```

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

mod base;
mod config;
mod metrics;
mod scale;

pub use base::{Base, Step};
pub use config::{Density, Incoherent, Sizing, TextScale, ThemeConfig, Tokens};
pub use metrics::{ControlSize, Metrics};
pub use scale::{ControlHeight, FontSize as TextSize, Gap, Radius, Scale};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Installs the default token resources. Idempotent — `init_resource`
/// only runs if the resource is absent, so consumers can pre-insert a
/// custom palette/spacing/etc. before adding [`ThemePlugin`].
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ThemeConfig>()
            .init_resource::<Tokens>()
            .init_resource::<ColorPalette>()
            .init_resource::<Spacing>()
            .init_resource::<Typography>()
            .init_resource::<FontHandle>();
    }
}

// ---------------------------------------------------------------------------
// Font handle
// ---------------------------------------------------------------------------

/// Font handles used for widget text. Two slots: regular and bold.
///
/// `regular = None` falls back to Bevy's built-in default font
/// (covers ASCII; misses ⌘ / ⌫ / arrow glyphs). `bold = None` falls
/// back to `regular` (so widgets that ask for bold text on a regular-
/// only setup just get the regular weight, which is what every
/// existing tempera example does).
///
/// Bevy_text's `TextFont.weight` only takes effect on **variable**
/// font files. To support headline-weight emphasis with regular .otf
/// or .ttf bundles (e.g. Inter-Bold.otf), set `bold` explicitly. The
/// [`Self::text_font_bold`] helper produces a `TextFont` with the
/// bold handle when one is set; widgets that want bold conditionally
/// call this instead of [`Self::text_font`].
#[derive(Resource, Clone, Default, Debug)]
pub struct FontHandle {
    pub regular: Option<Handle<Font>>,
    pub bold: Option<Handle<Font>>,
}

impl FontHandle {
    /// New `FontHandle` with only the regular slot filled.
    #[must_use]
    pub fn regular(handle: Handle<Font>) -> Self {
        Self {
            regular: Some(handle),
            bold: None,
        }
    }

    /// New `FontHandle` with both slots filled.
    #[must_use]
    pub fn with_bold(handle: Handle<Font>, bold: Handle<Font>) -> Self {
        Self {
            regular: Some(handle),
            bold: Some(bold),
        }
    }

    /// Build a [`TextFont`] using the regular handle at the given size,
    /// falling back to Bevy's default font if no handle is set.
    #[must_use]
    pub fn text_font(&self, size: f32) -> TextFont {
        let mut tf = TextFont {
            font_size: FontSize::Px(size),
            ..default()
        };
        if let Some(h) = &self.regular {
            tf.font = FontSource::Handle(h.clone());
        }
        tf
    }

    /// Build a bold [`TextFont`] using the bold handle if set, otherwise
    /// the regular handle.
    #[must_use]
    pub fn text_font_bold(&self, size: f32) -> TextFont {
        let mut tf = TextFont {
            font_size: FontSize::Px(size),
            ..default()
        };
        if let Some(h) = &self.bold {
            tf.font = FontSource::Handle(h.clone());
        } else if let Some(h) = &self.regular {
            tf.font = FontSource::Handle(h.clone());
        }
        tf
    }
}

// ---------------------------------------------------------------------------
// Color palette — shadcn naming, bevy `Color`s.
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Debug)]
pub struct ColorPalette {
    // Surfaces
    pub background: Color,
    pub card: Color,
    pub popover: Color,

    // Foregrounds
    pub foreground: Color,
    pub card_foreground: Color,
    pub popover_foreground: Color,

    // Action colors
    pub primary: Color,
    pub primary_foreground: Color,
    pub secondary: Color,
    pub secondary_foreground: Color,
    pub muted: Color,
    pub muted_foreground: Color,
    pub accent: Color,
    pub accent_foreground: Color,
    pub destructive: Color,
    pub destructive_foreground: Color,

    // Lines
    pub border: Color,
    pub input: Color,
    pub ring: Color,

    // States
    pub hover: Color,
    pub focus: Color,
}

impl ColorPalette {
    /// Lift a base color toward white by a small fixed amount — the
    /// canonical "hover" brightening. Use this when a widget needs a
    /// hover tint that doesn't already exist in the palette as its own
    /// token.
    #[inline]
    #[must_use]
    pub fn hover_lift(base: Color, amount: f32) -> Color {
        let s = base.to_srgba();
        Color::srgba(
            (s.red + amount).clamp(0.0, 1.0),
            (s.green + amount).clamp(0.0, 1.0),
            (s.blue + amount).clamp(0.0, 1.0),
            s.alpha,
        )
    }

    /// shadcn zinc dark palette.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            background: rgb(9, 9, 11),
            card: rgb(9, 9, 11),
            popover: rgb(17, 17, 20),

            foreground: rgb(250, 250, 250),
            card_foreground: rgb(250, 250, 250),
            popover_foreground: rgb(238, 238, 243),

            primary: rgb(250, 250, 250),
            primary_foreground: rgb(24, 24, 27),
            // secondary / muted / accent follow shadcn v4 dark: muted
            // is mid-grey, accent is a notably lighter grey so
            // selected-vs-hovered toggle items contrast against each
            // other.
            secondary: rgb(64, 64, 67),
            secondary_foreground: rgb(250, 250, 250),
            muted: rgb(64, 64, 67),
            muted_foreground: rgb(161, 161, 170),
            accent: rgb(95, 95, 99),
            accent_foreground: rgb(250, 250, 250),
            destructive: rgb(245, 90, 90),
            destructive_foreground: rgb(250, 250, 250),

            border: Color::srgba(1.0, 1.0, 1.0, 0.08),
            input: rgb(82, 82, 91),
            ring: rgb(212, 212, 216),

            hover: rgb(39, 39, 42),
            focus: rgb(250, 250, 250),
        }
    }

    /// shadcn zinc light palette.
    #[must_use]
    pub fn light() -> Self {
        Self {
            background: rgb(255, 255, 255),
            card: rgb(255, 255, 255),
            popover: rgb(255, 255, 255),

            foreground: rgb(9, 9, 11),
            card_foreground: rgb(9, 9, 11),
            popover_foreground: rgb(9, 9, 11),

            primary: rgb(24, 24, 27),
            primary_foreground: rgb(250, 250, 250),
            secondary: rgb(244, 244, 245),
            secondary_foreground: rgb(24, 24, 27),
            muted: rgb(244, 244, 245),
            muted_foreground: rgb(113, 113, 122),
            accent: rgb(244, 244, 245),
            accent_foreground: rgb(24, 24, 27),
            destructive: rgb(239, 68, 68),
            destructive_foreground: rgb(250, 250, 250),

            border: rgb(228, 228, 231),
            input: rgb(228, 228, 231),
            ring: rgb(24, 24, 27),

            hover: rgb(244, 244, 245),
            focus: rgb(24, 24, 27),
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark()
    }
}

#[inline]
fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::srgb_u8(r, g, b)
}

// ---------------------------------------------------------------------------
// Spacing scale (xxs..xxl + corner radii) — matches shadcn/Tailwind.
// ---------------------------------------------------------------------------

/// The spacing scale, as flat fields.
///
/// # These are cached, not authored
///
/// Every field here is [`Scale::at`] evaluated at a fixed [`Step`] — see
/// [`Self::from_scale`], which is the definition and the only way the values
/// are produced. They are fields rather than methods because ~17 call sites
/// read them by name today; the generator is the source of truth and this is
/// its cache.
///
/// **Do not assign to a field.** Setting `xs` without setting `sm` breaks the
/// proportionality the scale exists to maintain, and nothing will report it.
/// To change the spacing, change the [`Base`] and rebuild — that is what the
/// generator is for. (The fields will become private once the call sites
/// move to [`Scale::at`]; see PR 3 in the migration.)
#[derive(Resource, Clone, Debug)]
pub struct Spacing {
    /// [`Step`] −2.
    pub xxs: f32,
    /// [`Step`] 0 — the base itself.
    pub xs: f32,
    /// [`Step`] 2.
    pub sm: f32,
    /// [`Step`] 4.
    pub md: f32,
    /// [`Step`] 5.
    pub lg: f32,
    /// [`Step`] 6.
    pub xl: f32,
    /// [`Step`] 7.
    pub xxl: f32,

    /// [`Step`] −2.
    pub corner_radius_micro: f32,
    /// [`Step`] 0.
    pub corner_radius_tiny: f32,
    /// [`Step`] 1.
    pub corner_radius_small: f32,
    /// [`Step`] 2.
    pub corner_radius: f32,
    /// [`Step`] 3.
    pub corner_radius_large: f32,
}

impl Spacing {
    /// Evaluate the scale at the steps the named tokens sit on.
    ///
    /// The step assignments are the *only* place the ladder is declared. At
    /// the default base of 4 this reproduces the values tempera has always
    /// shipped — 2/4/8/16/24/32/48 and radii 2/4/6/8/12 — which is the check
    /// that the two-strand scale explains the existing design rather than
    /// replacing it. `radius_small` at 6 is exactly the ×3/2 strand, which
    /// is why that strand has to exist.
    #[must_use]
    pub fn from_scale(scale: Scale) -> Self {
        let at = |n: i8| scale.at(Step::new(n)).get();
        Self {
            xxs: at(-2),
            xs: at(0),
            sm: at(2),
            md: at(4),
            lg: at(5),
            xl: at(6),
            xxl: at(7),
            corner_radius_micro: at(-2),
            corner_radius_tiny: at(0),
            corner_radius_small: at(1),
            corner_radius: at(2),
            corner_radius_large: at(3),
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self::from_scale(Scale::new(Base::default()))
    }
}

// ---------------------------------------------------------------------------
// Typography scale (xxs..xxl).
// ---------------------------------------------------------------------------

/// The type ramp.
///
/// # Curated, not generated — and correctly so
///
/// Unlike [`Spacing`], these are not computed from a ratio, because type has
/// to survive hinting, x-height and optical size, none of which a ratio
/// models. Every major design system is algorithmic in spacing and hand-picked
/// in type; this is not an inconsistency to fix later.
///
/// (Note also that the 4px grid is near-universally applied to *line-height*
/// rather than to font-size. All 15 Material 3 line-heights divide by 4; the
/// sizes — 57/45/22/14/11 — do not.)
///
/// What [`TextScale`] does is shift the whole ramp by a notch, preserving the
/// hand-picked steps between its entries.
#[derive(Resource, Clone, Debug)]
pub struct Typography {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub base: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Typography {
    /// The ramp at a given text scale.
    ///
    /// [`TextScale::Medium`] is the ladder tempera has always shipped. The
    /// other two shift every entry by the same signed amount rather than
    /// multiplying, because the gaps between these sizes were chosen by eye
    /// at small sizes where a ratio would round badly (10 × 1.2 = 12, but
    /// 8 × 1.2 = 9.6).
    #[must_use]
    pub fn from_scale(text: TextScale) -> Self {
        // Values match the rendered pixel height egui produces with
        // these same numeric tokens, so tempera widgets look identical
        // to armas widgets when both share a Theme. Bevy_text and
        // egui both interpret `font_size` as logical pixels, but
        // armas-basic uses the slightly smaller scale (sm=12 vs
        // shadcn's 13) — we follow armas to keep dawai parity.
        let medium = Self {
            xxs: 8.0,
            xs: 10.0,
            sm: 12.0,
            base: 14.0,
            lg: 16.0,
            xl: 18.0,
            xxl: 24.0,
        };
        let shift = match text {
            TextScale::Small => -2.0,
            TextScale::Medium => return medium,
            TextScale::Large => 2.0,
        };
        Self {
            xxs: medium.xxs + shift,
            xs: medium.xs + shift,
            sm: medium.sm + shift,
            base: medium.base + shift,
            lg: medium.lg + shift,
            xl: medium.xl + shift,
            xxl: medium.xxl + shift,
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self::from_scale(TextScale::default())
    }
}

// ---------------------------------------------------------------------------
// Reacting to a token change
// ---------------------------------------------------------------------------

/// Run condition: a token resource changed this frame.
///
/// Repaint systems filter their queries on `Changed<Interaction>` so they
/// touch only the widget the pointer moved over — which is right, and which
/// also means a palette swap repaints *nothing* until the user happens to
/// hover each widget in turn. A query filter cannot express "or the theme
/// moved", because that is not a fact about any entity.
///
/// So the systems drop the filter and gate on this instead: they run when a
/// widget's interaction changed **or** when the palette did, and they
/// compare before writing so the unchanged case stays free.
///
/// Deliberately reads only [`ColorPalette`] — the geometry tokens are
/// consumed at spawn and are not repaint inputs. Adding them here would
/// make every hover repaint depend on resources it never reads.
pub fn palette_changed(palette: Res<ColorPalette>) -> bool {
    palette.is_changed()
}

/// Run condition for a widget's repaint system: something it paints from
/// moved.
///
/// Two independent triggers, which is the whole reason this exists:
///
/// - a widget's own `Interaction` changed, or one was just spawned — the
///   per-entity case the old `Or<(Changed<Interaction>, Added<W>)>` query
///   filter covered; and
/// - [`ColorPalette`] changed, which makes **every** widget stale at once.
///
/// The second cannot be a query filter. `Changed<T>` asks a question about
/// an entity, and "the theme moved" is not a fact about any entity — which
/// is why a palette swap used to repaint nothing until the user hovered
/// each widget in turn.
///
/// Moving the test from the query to the system means the system's query
/// is unfiltered and it iterates every widget of its kind on the frames it
/// does run. That is affordable only because each of these systems compares
/// before writing, so an unchanged widget costs a `Color` comparison and
/// marks nothing dirty.
pub fn repaint_needed<W: Component>(
    palette: Res<ColorPalette>,
    touched: Query<(), Or<(Changed<Interaction>, Added<W>)>>,
) -> bool {
    palette.is_changed() || !touched.is_empty()
}

/// [`repaint_needed`] for a widget with extra state of its own.
///
/// A checkbox restyles on `Changed<Checked>` and a switch on
/// `Changed<InteractionDisabled>` as well as on hover, and those triggers
/// have to survive the move out of the query filter — dropping one would
/// leave a checkbox showing the wrong art until the pointer crossed it.
///
/// `F` is the widget's own trigger set, exactly as it read inside the old
/// `Or<(..)>`; the palette and `Added<W>` are supplied here.
///
/// ```ignore
/// repaint_needed_on::<TemperaCheckbox, Or<(Changed<Checked>, Changed<InteractionDisabled>)>>
/// ```
pub fn repaint_needed_on<W: Component, F: QueryFilter + 'static>(
    palette: Res<ColorPalette>,
    touched: Query<(), Or<(Changed<Interaction>, Added<W>, F)>>,
) -> bool {
    palette.is_changed() || !touched.is_empty()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generated_spacing_reproduces_the_shipped_values() {
        // The claim the whole model rests on: the two-strand scale from base
        // 4 *explains* the ladder tempera already had, rather than replacing
        // it. If this fails, either a step assignment in `from_scale` drifted
        // or the generator did — and either way something on screen moved
        // that this PR promised not to move.
        let s = Spacing::default();
        assert_eq!(
            (s.xxs, s.xs, s.sm, s.md, s.lg, s.xl, s.xxl),
            (2.0, 4.0, 8.0, 16.0, 24.0, 32.0, 48.0)
        );
        assert_eq!(
            (
                s.corner_radius_micro,
                s.corner_radius_tiny,
                s.corner_radius_small,
                s.corner_radius,
                s.corner_radius_large,
            ),
            (2.0, 4.0, 6.0, 8.0, 12.0)
        );
    }

    #[test]
    fn the_odd_radius_is_the_second_strand() {
        // `corner_radius_small` is 6, which is not on the doubling strand at
        // all — it is base × 3/2. It is the single clearest piece of evidence
        // that the existing design was already two-stranded, and the reason
        // a one-strand scale could not have been retrofitted here.
        let scale = Scale::new(Base::FOUR);
        assert_eq!(scale.at(Step::new(1)).get(), 6.0);
        assert_eq!(Spacing::default().corner_radius_small, 6.0);
    }

    #[test]
    fn a_coarser_base_scales_the_whole_ladder() {
        // What the model buys: one input moves, everything follows in
        // proportion, and nothing needs re-tuning by hand.
        let s = Spacing::from_scale(Scale::new(Base::EIGHT));
        let four = Spacing::default();
        assert_eq!(s.xs, four.xs * 2.0);
        assert_eq!(s.sm, four.sm * 2.0);
        assert_eq!(s.md, four.md * 2.0);
        assert_eq!(s.corner_radius_small, four.corner_radius_small * 2.0);
    }

    #[test]
    fn the_default_typography_is_unchanged() {
        let t = Typography::default();
        assert_eq!(
            (t.xxs, t.xs, t.sm, t.base, t.lg, t.xl, t.xxl),
            (8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 24.0)
        );
    }

    #[test]
    fn the_text_scale_shifts_the_ramp_and_keeps_its_steps() {
        // A shift, not a multiply — the gaps between these hand-picked sizes
        // are the curation, so they have to survive the move.
        let med = Typography::from_scale(TextScale::Medium);
        let big = Typography::from_scale(TextScale::Large);
        assert_eq!(big.base - med.base, 2.0);
        assert_eq!(big.sm - med.sm, big.lg - med.lg);

        let small = Typography::from_scale(TextScale::Small);
        assert!(small.xxs > 0.0, "no entry may shift to zero or below");
    }

    #[test]
    fn the_theme_plugin_installs_a_coherent_config() {
        let mut app = App::new();
        app.add_plugins(ThemePlugin);
        let tokens = *app.world().resource::<Tokens>();
        let config = *app.world().resource::<ThemeConfig>();
        assert_eq!(tokens.config, config);
        assert_eq!(
            app.world().resource::<Spacing>().xs,
            tokens.scale.at(Step::BASE).get()
        );
    }

    #[test]
    fn a_pre_inserted_config_survives_the_plugin() {
        // `init_resource` is a no-op when the resource is present, which is
        // what lets a host override the theme before adding the plugin. The
        // whole point of a config resource is lost if the plugin stomps it.
        let mut app = App::new();
        let dense = ThemeConfig {
            base: Base::EIGHT,
            density: Density::Compact,
            text: TextScale::Small,
        };
        app.insert_resource(dense).add_plugins(ThemePlugin);
        assert_eq!(*app.world().resource::<ThemeConfig>(), dense);
    }
}
