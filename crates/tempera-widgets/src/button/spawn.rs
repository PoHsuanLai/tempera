//! Spawn helper + the `ButtonStyle` system-param bundle.
//!
//! `ButtonStyle` is the curated read-only slice of theme tokens that
//! button paint / spawn code needs. Other widgets follow the same
//! pattern (`SliderStyle`, `CheckboxStyle`, …) so each widget surfaces
//! its theme dependencies at the type level.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::{ButtonSize, ButtonVariant};
use crate::theme::{ColorPalette, FontHandle, Metrics, Spacing, Typography};

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
#[derive(Clone, Debug)]
pub enum ButtonContent {
    Text(String),
    Icon(Handle<Image>),
    TextAndIcon { text: String, icon: Handle<Image> },
}

impl ButtonContent {
    #[must_use]
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    #[must_use]
    pub fn icon(handle: Handle<Image>) -> Self {
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
        // shadcn buttons are intrinsic-width. Without `FlexStart` here,
        // a column-flex parent's default `align_items: Stretch` blows
        // the button out to full row width.
        align_self: AlignSelf::FlexStart,
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
    image: Handle<Image>,
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
    parent.spawn((
        ImageNode::new(image),
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
#[derive(Clone, Copy, Debug)]
pub(crate) struct VariantVisuals {
    pub bg_resting: Color,
    pub bg_hover: Color,
    pub bg_pressed: Color,
    pub border_resting: Color,
    pub border_width: f32,
    pub fg_resting: Color,
}

impl VariantVisuals {
    /// Fill for a selected-but-unhovered button.
    ///
    /// Derived rather than declared per variant, because "selected" is the
    /// same visual idea everywhere and seven hand-picked colours would be
    /// seven more numbers to keep in agreement.
    ///
    /// **It is deliberately not `bg_hover`.** The tempting definition is
    /// "hold the state the hover would have shown", and it is wrong: for the
    /// transparent variants `bg_hover` *is* `muted`, so a selected button
    /// would already be sitting at its hovered colour and the pointer would
    /// produce no change at all. A selected control that stops reacting reads
    /// as a disabled one. Hence a distinct step, below hover, that hover can
    /// still lift away from.
    ///
    /// A variant with no surface to lift — `Bare`, `Link` — starts from
    /// `muted` instead: those exist to be invisible, but a *selected* one
    /// still has to be distinguishable from its neighbours.
    pub(crate) fn bg_selected(&self, palette: &ColorPalette) -> Color {
        let base = if self.bg_resting == Color::NONE {
            palette.muted
        } else {
            self.bg_resting
        };
        ColorPalette::step(base, palette.background, 0.04)
    }
}

/// A button's hover fill: one step away from the page.
///
/// Wraps [`ColorPalette::step`] only to name the surface once — every button
/// in tempera sits on `background`, and a variant that sits on a card or a
/// popover would pass its own surface here instead.
fn hover(base: Color, palette: &ColorPalette) -> Color {
    ColorPalette::step(base, palette.background, 0.08)
}

/// A button's pressed fill: a **further** step in the same direction as
/// [`hover`].
///
/// Press is only ever seen as a change *from* the hovered colour, because the
/// pointer is already on the button — so what has to be visible is
/// hover → pressed, not resting → pressed.
///
/// The obvious alternative is to reverse direction, which is what the old
/// `lighten`-hover / `darken`-pressed pairing did. That reads well on a dark
/// palette and breaks on a light one, in a way worth recording: reversing
/// means moving *toward* the surface, and for a fill already close to it
/// there is nowhere to go. `secondary` in the light theme is `(244,244,245)`
/// on a `(255,255,255)` page — a reversed pressed step lands exactly on
/// `(255,255,255)`, so pressing a secondary button would erase it. Stepping
/// further away cannot do that, because the direction is the one chosen for
/// having room in it.
fn pressed(base: Color, palette: &ColorPalette) -> Color {
    ColorPalette::step(base, palette.background, 0.14)
}

pub(crate) fn variant_visuals(variant: ButtonVariant, palette: &ColorPalette) -> VariantVisuals {
    match variant {
        ButtonVariant::Default => VariantVisuals {
            bg_resting: palette.primary,
            bg_hover: hover(palette.primary, palette),
            bg_pressed: pressed(palette.primary, palette),
            border_resting: Color::NONE,
            border_width: 0.0,
            fg_resting: palette.primary_foreground,
        },
        ButtonVariant::Secondary => VariantVisuals {
            bg_resting: palette.secondary,
            bg_hover: hover(palette.secondary, palette),
            bg_pressed: pressed(palette.secondary, palette),
            border_resting: Color::NONE,
            border_width: 0.0,
            fg_resting: palette.secondary_foreground,
        },
        ButtonVariant::Outline => VariantVisuals {
            bg_resting: Color::NONE,
            bg_hover: palette.muted,
            bg_pressed: pressed(palette.muted, palette),
            border_resting: palette.border,
            border_width: 1.0,
            fg_resting: palette.foreground,
        },
        ButtonVariant::Ghost => VariantVisuals {
            bg_resting: Color::NONE,
            bg_hover: palette.muted,
            bg_pressed: pressed(palette.muted, palette),
            border_resting: Color::NONE,
            border_width: 0.0,
            fg_resting: palette.foreground,
        },
        ButtonVariant::Bare => VariantVisuals {
            bg_resting: Color::NONE,
            bg_hover: Color::NONE,
            bg_pressed: Color::NONE,
            border_resting: Color::NONE,
            border_width: 0.0,
            fg_resting: palette.foreground,
        },
        ButtonVariant::Link => VariantVisuals {
            bg_resting: Color::NONE,
            bg_hover: Color::NONE,
            bg_pressed: Color::NONE,
            border_resting: Color::NONE,
            border_width: 0.0,
            fg_resting: palette.primary,
        },
        ButtonVariant::Destructive => VariantVisuals {
            bg_resting: palette.destructive,
            bg_hover: hover(palette.destructive, palette),
            bg_pressed: pressed(palette.destructive, palette),
            border_resting: Color::NONE,
            border_width: 0.0,
            fg_resting: palette.destructive_foreground,
        },
    }
}


