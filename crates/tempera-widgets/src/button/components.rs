//! Small components describing a button's appearance.
//!
//! Variant and size are independent: insert / remove either without
//! affecting the other. The paint system reads whichever is present
//! and falls back to [`ButtonVariant::Default`] / [`ButtonSize::Md`].

use bevy::prelude::*;

use crate::theme::{ControlSize, Metrics};

/// Tempera marker on `spawn_button` outputs. Differentiates a
/// tempera-styled button from any other entity that uses
/// [`bevy::ui_widgets::Button`] purely for click behavior (tab
/// triggers, dropdown triggers, etc.). The paint system filters on
/// this marker so we don't accidentally repaint every Button-bearing
/// entity in the app with the default primary-color fill.
#[derive(Component, Default, Debug)]
pub struct TemperaButton;

/// Opt-in tinting for an icon child. When present on a tempera
/// button, the paint system writes `ImageNode.color` on the button's
/// icon child between `resting` (idle) and `hover` (hovered/pressed).
/// Pair with [`crate::button::ButtonVariant::Ghost`] to get dawai's
/// toolbar-glyph pattern: no surface fill, icon recolors on hover.
///
/// Expects the icon to be rasterized as a tintable (typically white)
/// PNG so the `ImageNode.color` multiplication actually shows.
#[derive(Component, Clone, Copy, Debug)]
pub struct IconTint {
    pub resting: Color,
    pub hover: Color,
}

impl IconTint {
    pub const fn new(resting: Color, hover: Color) -> Self {
        Self { resting, hover }
    }
}

/// Visual variant of a button. Maps to a color recipe applied by the
/// paint system using the [`crate::ColorPalette`] resource.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Filled, primary-color background. The default if no variant is
    /// inserted on the button entity.
    #[default]
    Default,
    /// Filled, secondary (muted) background.
    Secondary,
    /// Transparent background, 1px border.
    Outline,
    /// Transparent background, no border — paints on hover only.
    Ghost,
    /// Transparent everywhere — no surface fill on hover or press. Pair
    /// with [`IconTint`] for icon-only action glyphs that only shift
    /// tint on hover (Plasticity's inline outliner action buttons).
    Bare,
    /// Text-only, underline on hover.
    Link,
    /// Filled with `destructive` color (delete / danger actions).
    Destructive,
}

/// Sizing preset for a button. Controls height, horizontal padding,
/// and text size. The `Icon*` variants are square with no horizontal
/// padding — pair them with [`super::ButtonContent::Icon`] for
/// shadcn-style icon-only buttons.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// 22px tall — tight chips / inline controls.
    Xs,
    /// 28px tall — compact dense toolbars.
    Sm,
    /// 32px tall — default for most buttons.
    #[default]
    Md,
    /// 40px tall — large CTAs.
    Lg,
    /// 32×32 square — shadcn `size="icon"`. Use with `Icon` content.
    Icon,
    /// 24×24 square — shadcn `size="icon-sm"`. Use with `Icon` content.
    IconSm,
}

impl ButtonSize {
    /// Which declared control height this size maps onto, if any.
    ///
    /// Three of the six do. `Sm` is [`ControlSize::Sm`] and `Md`/`Icon` are
    /// [`ControlSize::Md`] — the same quantity `Sizing` declares, previously
    /// restated here as a literal with nothing linking the two.
    ///
    /// The other three return `None`, and that is a finding rather than
    /// unfinished work. `Lg` is 40 while [`Sizing::control_lg`] is 44; `Xs`
    /// is 22 and `IconSm` is 24. Mapping them anyway would move pixels, and
    /// inventing a fourth and fifth `ControlSize` to fit would make the enum
    /// a list of every height in use rather than a set of alignment classes.
    /// They stay declared, in the table below, where a reader can see them.
    #[inline]
    #[must_use]
    pub const fn control_size(self) -> Option<ControlSize> {
        match self {
            Self::Sm => Some(ControlSize::Sm),
            Self::Md | Self::Icon => Some(ControlSize::Md),
            Self::Xs | Self::Lg | Self::IconSm => None,
        }
    }

    /// Total button height (and width, for icon sizes), resolved against
    /// the theme.
    ///
    /// Prefer this over [`Self::height`] in new code: it reads the declared
    /// height for the sizes that have one, so a density change reaches them.
    #[inline]
    #[must_use]
    pub fn height_from(self, metrics: &Metrics) -> f32 {
        match self.control_size() {
            Some(size) => metrics.control(size).get(),
            None => self.height(),
        }
    }

    /// Total button height (and width, for icon sizes) in logical
    /// pixels.
    ///
    /// The fallback table. [`Self::height_from`] is the theme-aware
    /// accessor; this stays because three of the six sizes are genuinely
    /// declared here (see [`Self::control_size`]) and because it is `const`,
    /// which several call sites still rely on.
    #[inline]
    #[must_use]
    pub const fn height(self) -> f32 {
        match self {
            Self::Xs => 22.0,
            Self::Sm => 28.0,
            Self::Md => 32.0,
            Self::Lg => 40.0,
            Self::Icon => 32.0,
            Self::IconSm => 24.0,
        }
    }

    /// Horizontal padding on each side. Icon sizes are square and
    /// zero-padded so the icon fills the widget.
    #[inline]
    #[must_use]
    pub const fn padding_x(self) -> f32 {
        match self {
            Self::Xs => 8.0,
            Self::Sm => 10.0,
            Self::Md => 14.0,
            Self::Lg => 18.0,
            Self::Icon | Self::IconSm => 0.0,
        }
    }

    /// True for the square icon-only sizes. The spawn helper sets an
    /// explicit `width = height` on these so flex doesn't size them
    /// from icon content alone.
    #[inline]
    #[must_use]
    pub const fn is_icon(self) -> bool {
        matches!(self, Self::Icon | Self::IconSm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics_app() -> App {
        let mut app = App::new();
        app.add_plugins(crate::theme::ThemePlugin);
        app
    }

    fn height_of(app: &mut App, size: ButtonSize) -> f32 {
        let mut state: bevy::ecs::system::SystemState<Metrics> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let metrics = state.get(app.world()).expect("theme resources present");
        size.height_from(&metrics)
    }

    #[test]
    fn the_token_path_agrees_with_the_table_today() {
        // The migration is only safe if it changes nothing at the default
        // config. Every size, not just the mapped ones — an accidental
        // mapping of `Lg` to `control_lg` would move it 40 → 44, and this
        // is what catches that.
        let mut app = metrics_app();
        for size in [
            ButtonSize::Xs,
            ButtonSize::Sm,
            ButtonSize::Md,
            ButtonSize::Lg,
            ButtonSize::Icon,
            ButtonSize::IconSm,
        ] {
            assert_eq!(
                height_of(&mut app, size),
                size.height(),
                "{size:?} moved when it should not have"
            );
        }
    }

    #[test]
    fn a_mapped_size_follows_the_theme_and_an_unmapped_one_does_not() {
        // The whole point of the split. `Md` is `control_md`, so a density
        // change reaches it; `Lg` is 40, which is nobody's declared height,
        // so it stays where it was declared.
        let mut app = App::new();
        let spacious = crate::theme::ThemeConfig {
            density: crate::theme::Density::Spacious,
            ..default()
        };
        app.insert_resource(spacious)
            .insert_resource(spacious.build().expect("spacious is coherent"))
            .add_plugins(crate::theme::ThemePlugin);

        assert_eq!(
            height_of(&mut app, ButtonSize::Md),
            40.0,
            "a mapped size must follow the density"
        );
        assert_eq!(
            height_of(&mut app, ButtonSize::Lg),
            ButtonSize::Lg.height(),
            "an unmapped size must stay declared"
        );
    }

    #[test]
    fn every_mapped_size_matches_the_height_it_claims() {
        // `control_size` asserts an identity — that `ButtonSize::Sm` *is*
        // `Sizing::control_sm`. If the two ever differ at the default
        // config, the mapping is a lie and buttons stop aligning with the
        // inputs beside them.
        let tokens = crate::theme::ThemeConfig::default().build().unwrap();
        for size in [ButtonSize::Sm, ButtonSize::Md, ButtonSize::Icon] {
            let claimed = size.control_size().expect("mapped");
            assert_eq!(tokens.sizing.get(claimed).get(), size.height(), "{size:?}");
        }
    }

    #[test]
    fn an_icon_button_stays_square_at_every_density() {
        // `Icon` maps onto `control_md`, so its height follows the theme
        // while its width is written separately in `spawn_button_sized`.
        // Those two must move together: reading the `const` table for one
        // and the token for the other leaves a 40px-tall icon button 32px
        // wide, which no colour or layout test would have noticed.
        for density in [
            crate::theme::Density::Compact,
            crate::theme::Density::Comfortable,
            crate::theme::Density::Spacious,
        ] {
            let config = crate::theme::ThemeConfig {
                density,
                ..default()
            };
            let mut app = App::new();
            app.insert_resource(config)
                .insert_resource(config.build().expect("coherent"))
                .add_plugins(crate::theme::ThemePlugin);

            let mut state: bevy::ecs::system::SystemState<(Commands, crate::button::ButtonStyle)> =
                bevy::ecs::system::SystemState::new(app.world_mut());
            let entity = {
                let (mut commands, style) = state.get(app.world()).expect("theme present");
                crate::button::spawn_button_sized(
                    &mut commands,
                    &style,
                    crate::button::ButtonContent::text(""),
                    ButtonVariant::Ghost,
                    ButtonSize::Icon,
                )
            };
            state.apply(app.world_mut());

            let node = app.world().get::<Node>(entity).expect("spawned");
            assert_eq!(
                node.width, node.height,
                "an icon button must be square at {density:?}"
            );
        }
    }
}
