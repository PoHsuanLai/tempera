//! Spawn helper + the `ButtonStyle` system-param bundle.
//!
//! `ButtonStyle` is the curated read-only slice of theme tokens that
//! button paint / spawn code needs. Other widgets follow the same
//! pattern (`SliderStyle`, `CheckboxStyle`, …) so each widget surfaces
//! its theme dependencies at the type level.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_resvg::prelude::{SvgFile, UiSvg};

use super::components::{ButtonSize, ButtonVariant};
use crate::theme::{
    ColorPalette, Emphasis, FontHandle, Metrics, Reactivity, Spacing, Surface, Typography, visuals,
};

/// The slice of theme tokens read by button systems and the
/// [`spawn_button`] helper.
#[derive(SystemParam)]
pub struct ButtonStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub spacing: Res<'w, Spacing>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    /// The geometry half. Sizes resolve through here rather than through
    /// [`ButtonSize`]'s `const` table, so the three sizes that map onto a
    /// declared [`crate::theme::Sizing`] height follow a density change.
    pub metrics: Metrics<'w>,
}

impl<'w> ButtonStyle<'w> {
    /// Build the text font for a button of the given size.
    #[must_use]
    pub fn text_font(&self, size: ButtonSize) -> TextFont {
        let pt = match size {
            ButtonSize::Xs => self.typography.xs,
            ButtonSize::Sm | ButtonSize::Md | ButtonSize::IconSm => self.typography.sm,
            ButtonSize::Lg => self.typography.base,
            // Icon-only buttons have no text child, but a font is
            // still required by the spawn helper signature — pick
            // the regular size.
            ButtonSize::Icon => self.typography.sm,
        };
        self.font.text_font(pt)
    }
}

/// What goes inside a button.
///
/// Text and Icon are mutually exclusive in the common case; for an
/// icon-with-label use [`ButtonContent::TextAndIcon`].
///
/// Icons are SVG. A widget glyph is monochrome line art that scales with
/// `Metrics` and tints from the palette — a vector answers all three, a
/// bitmap answers none. `asset_server.load("icons/search.svg")` gives the
/// `Handle<SvgFile>` this wants.
#[derive(Clone, Debug)]
pub enum ButtonContent {
    Text(String),
    Icon(Handle<SvgFile>),
    TextAndIcon { text: String, icon: Handle<SvgFile> },
}

impl ButtonContent {
    #[must_use]
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    #[must_use]
    pub fn icon(handle: Handle<SvgFile>) -> Self {
        Self::Icon(handle)
    }
}

/// Spawn a styled button entity. Returns the root entity so callers
/// can attach observers (`commands.entity(id).observe(...)`).
///
/// The returned entity carries `Button` (behavior), the supplied
/// `variant`, the default `ButtonSize::Md`, and a `Node` styled per
/// variant. Add [`ButtonSize`] explicitly to override the size, or
/// [`bevy::ui::InteractionDisabled`] to disable.
pub fn spawn_button(
    commands: &mut Commands,
    style: &ButtonStyle,
    content: ButtonContent,
    variant: ButtonVariant,
) -> Entity {
    spawn_button_sized(commands, style, content, variant, ButtonSize::Md)
}

/// Like [`spawn_button`] but with an explicit size — use this for
/// icon-only buttons (`ButtonSize::Icon` / `IconSm`) or for any
/// non-default sizing.
pub fn spawn_button_sized(
    commands: &mut Commands,
    style: &ButtonStyle,
    content: ButtonContent,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Entity {
    let visuals = variant_visuals(variant, &style.palette);
    let height = size.height_from(&style.metrics);

    let mut node = Node {
        height: Val::Px(height),
        padding: UiRect::horizontal(Val::Px(size.padding_x())),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(style.spacing.xs),
        border: UiRect::all(Val::Px(visuals.border_width)),
        border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small)),
        // shadcn buttons are intrinsic-width: without an explicit
        // `align_self` a column-flex parent's default `align_items: Stretch`
        // blows the button out to the parent's full width.
        //
        // `Center` and not `FlexStart`, because this one property means two
        // different things depending on the parent. In a column it is the
        // horizontal size question above, and either value answers it. In a
        // *row* it is vertical position, and `FlexStart` pins the button to
        // the top edge — overriding the parent's own `align_items: Center`,
        // which is what a toolbar sets precisely to avoid that. A 24px button
        // in a 40px bar sat 8px high, with the row itself already correct.
        align_self: AlignSelf::Center,
        ..default()
    };
    if size.is_icon() {
        // Square: explicit width keeps the button from collapsing to
        // the icon's intrinsic size on its own. Must be the *resolved*
        // height, not `size.height()` — `Icon` maps onto `control_md`, so
        // reading the table here would leave a 40px-tall icon button 32px
        // wide at Spacious density.
        node.width = Val::Px(height);
    }

    let mut root = commands.spawn((
        super::Button,
        super::TemperaButton,
        variant,
        size,
        node,
        BackgroundColor(visuals.bg_resting),
        BorderColor::all(visuals.border_resting),
        // bevy_ui's `Node` doesn't require `Interaction` and
        // bevy_ui_widgets's headless `Button` doesn't either. Without
        // this component, the picking observers can't track
        // hover/press state — the paint system's `&Interaction`
        // filter would skip the button and hover styling never fires.
        Interaction::default(),
        crate::cursor::HoverCursor::default(),
        Name::new("tempera::button"),
    ));

    let id = root.id();

    root.with_children(|parent| match content {
        ButtonContent::Text(text) => {
            spawn_text(parent, style, &text, size, visuals.fg_resting);
        }
        ButtonContent::Icon(image) => {
            spawn_icon(parent, image, size, height);
        }
        ButtonContent::TextAndIcon { text, icon } => {
            spawn_icon(parent, icon, size, height);
            spawn_text(parent, style, &text, size, visuals.fg_resting);
        }
    });

    id
}

fn spawn_text(
    parent: &mut ChildSpawnerCommands,
    style: &ButtonStyle,
    text: &str,
    size: ButtonSize,
    color: Color,
) {
    parent.spawn((
        Text::new(text),
        style.text_font(size),
        TextColor(color),
        bevy::picking::Pickable::IGNORE,
    ));
}

fn spawn_icon(
    parent: &mut ChildSpawnerCommands,
    image: Handle<SvgFile>,
    size: ButtonSize,
    height: f32,
) {
    // Icon-only buttons devote ~62% of the widget to the icon
    // (matches shadcn `[&_svg:not([class*='size-'])]:size-4` on a
    // 24px or 32px button). Text-bearing buttons keep the smaller
    // 50% icon so it sits visually balanced next to the label.
    //
    // `height` is the resolved height, not `size.height()`, so an icon
    // stays proportional when a density change moves its button.
    let icon_size = if size.is_icon() {
        (height * 0.62).floor()
    } else {
        (height * 0.5).floor()
    };
    // `UiSvg`, not `ImageNode`: `bevy_resvg` inserts the `ImageNode` once
    // the asset lands, and its query is filtered `Without<ImageNode>`. An
    // `ImageNode` written here would make the icon invisible — the plugin
    // would skip the entity and never fill in the rasterised image.
    parent.spawn((
        UiSvg(image),
        Node {
            width: Val::Px(icon_size),
            height: Val::Px(icon_size),
            ..default()
        },
        bevy::picking::Pickable::IGNORE,
    ));
}

// ---------------------------------------------------------------------------
// Variant → color recipe
// ---------------------------------------------------------------------------

/// Visual state derived from a variant + the palette. Computed once per
/// repaint by the style-sync system.
///
/// A thin renaming of [`SurfaceVisuals`] into the button's own vocabulary.
/// The recipe itself lives in `tempera-theme` because it is not a button
/// fact — a text input and a chip want the same fills — and the button's job
/// is only to say which point on the grid each of its variants sits at.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VariantVisuals {
    pub bg_resting: Color,
    pub bg_hover: Color,
    pub bg_pressed: Color,
    pub bg_selected: Color,
    pub border_resting: Color,
    pub border_width: f32,
    pub fg_resting: Color,
}

impl ButtonVariant {
    /// Where this variant sits on the [`Surface`] × [`Emphasis`] ×
    /// [`Reactivity`] grid.
    ///
    /// `Link` returns `None`: it is fill-less, edge-less, draws `primary` as
    /// *text* and underlines on hover — a text treatment wearing a button's
    /// clothes. It is handled separately rather than by widening the grid to
    /// fit one member.
    fn grid(self) -> Option<(Surface, Emphasis, Reactivity)> {
        use Emphasis as E;
        use Reactivity as R;
        use Surface as S;
        Some(match self {
            Self::Default => (S::Filled, E::Primary, R::Fills),
            Self::Secondary => (S::Filled, E::Secondary, R::Fills),
            Self::Destructive => (S::Filled, E::Destructive, R::Fills),
            Self::Outline => (S::Outline, E::Neutral, R::Fills),
            Self::Ghost => (S::Bare, E::Neutral, R::Fills),
            // Identical to Ghost but for the pointer: a bare button in a
            // dense toolbar pairs with `IconTint`, and a fill would flicker
            // as the pointer crossed the row.
            Self::Bare => (S::Bare, E::Neutral, R::Inert),
            Self::Link => return None,
        })
    }
}

pub(crate) fn variant_visuals(variant: ButtonVariant, palette: &ColorPalette) -> VariantVisuals {
    let Some((surface, emphasis, reactivity)) = variant.grid() else {
        // `Link`: text-only, no surface in any state, underlined on hover by
        // the paint system rather than by a colour.
        return VariantVisuals {
            bg_resting: Color::NONE,
            bg_hover: Color::NONE,
            bg_pressed: Color::NONE,
            bg_selected: ColorPalette::step(palette.muted, palette.background, 0.04),
            border_resting: Color::NONE,
            border_width: 0.0,
            fg_resting: palette.primary,
        };
    };

    // Buttons sit on the page. A button hosted inside a card or a popover
    // would pass that surface instead, which is the argument's whole point.
    let v = visuals(surface, emphasis, reactivity, palette, palette.background);
    VariantVisuals {
        bg_resting: v.fill,
        bg_hover: v.fill_hover,
        bg_pressed: v.fill_pressed,
        bg_selected: v.fill_selected,
        border_resting: v.edge,
        border_width: v.edge_width,
        fg_resting: v.text,
    }
}

#[cfg(test)]
mod recipe_tests {
    use super::*;
    use crate::theme::ColorPalette;

    /// Every variant's resolved colours, before and after the grid refactor.
    ///
    /// The grid is a *re-expression*, not a redesign: it had to reproduce
    /// what `variant_visuals` already returned, arm for arm, or it would be
    /// smuggling a visual change into a structural PR. These are the values
    /// the hand-written seven-arm match produced, transcribed at the point of
    /// the rewrite.
    ///
    /// If a later change to `Surface`/`Emphasis` moves one of these, that is
    /// a visual decision and belongs in its own reviewed change — this test
    /// is what makes it impossible to make one by accident.
    #[test]
    fn the_grid_reproduces_every_variant_it_replaced() {
        let p = ColorPalette::dark();
        let case = |variant: ButtonVariant| {
            let v = variant_visuals(variant, &p);
            (
                v.bg_resting,
                v.bg_hover,
                v.bg_pressed,
                v.border_resting,
                v.border_width,
                v.fg_resting,
            )
        };

        let step = |c: Color, a: f32| ColorPalette::step(c, p.background, a);

        assert_eq!(
            case(ButtonVariant::Default),
            (
                p.primary,
                step(p.primary, 0.08),
                step(p.primary, 0.14),
                Color::NONE,
                0.0,
                p.primary_foreground
            )
        );
        assert_eq!(
            case(ButtonVariant::Secondary),
            (
                p.secondary,
                step(p.secondary, 0.08),
                step(p.secondary, 0.14),
                Color::NONE,
                0.0,
                p.secondary_foreground
            )
        );
        assert_eq!(
            case(ButtonVariant::Destructive),
            (
                p.destructive,
                step(p.destructive, 0.08),
                step(p.destructive, 0.14),
                Color::NONE,
                0.0,
                p.destructive_foreground
            )
        );
        assert_eq!(
            case(ButtonVariant::Outline),
            (
                Color::NONE,
                p.muted,
                step(p.muted, 0.08),
                p.border,
                1.0,
                p.foreground
            )
        );
        assert_eq!(
            case(ButtonVariant::Ghost),
            (
                Color::NONE,
                p.muted,
                step(p.muted, 0.08),
                Color::NONE,
                0.0,
                p.foreground
            )
        );
        assert_eq!(
            case(ButtonVariant::Bare),
            (
                Color::NONE,
                Color::NONE,
                Color::NONE,
                Color::NONE,
                0.0,
                p.foreground
            )
        );
        assert_eq!(
            case(ButtonVariant::Link),
            (
                Color::NONE,
                Color::NONE,
                Color::NONE,
                Color::NONE,
                0.0,
                p.primary
            )
        );
    }
}
