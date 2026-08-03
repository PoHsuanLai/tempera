use bevy::ecs::system::SystemParam;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy_ui_text_input::actions::{TextInputAction, TextInputEdit};
use bevy_ui_text_input::{
    TextInputContents, TextInputMode, TextInputNode, TextInputPrompt, TextInputQueue,
    TextInputStyle as UpstreamCursorStyle,
};

use super::components::TextInput;
use crate::theme::{
    ColorPalette, ControlSize, FontHandle, Metrics, Spacing, Step, StyledNode, Typography,
};

/// The height an input used to declare for itself, kept as a reference
/// value.
///
/// **Nothing lays out from this any more** — the live height comes from
/// [`ControlSize::Md`], because an input is a control and its height is
/// declared once for all controls rather than here.
///
/// It survives as the fixed point in
/// `an_input_is_as_tall_as_the_declared_control`: if a future edit to
/// `Density`'s multiplier table moves `control_md` off 32, that test fails
/// and someone decides deliberately, rather than the input quietly
/// desynchronising from the buttons beside it. Deleting it would delete
/// that check.
#[cfg(test)]
pub(crate) const HEIGHT: f32 = 32.0;

/// A **content measure**, not a grid value.
///
/// 240px is how wide a single-line input wants to be for the text people
/// type into it. No spacing scale predicts that, and snapping it to one
/// would be false precision — see [`Metrics::measure`].
pub(crate) const DEFAULT_WIDTH: f32 = 240.0;

#[derive(SystemParam)]
pub struct TextInputStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub spacing: Res<'w, Spacing>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
    /// The geometry half — see [`Metrics`].
    pub metrics: Metrics<'w>,
}

/// Returned from [`spawn_text_input`] — the two entities that make up
/// the widget.
#[derive(Clone, Copy, Debug)]
pub struct TextInputHandle {
    /// Tempera-styled root carrying the [`TextInput`] marker, border,
    /// background, and flex layout that vertically centers the inner.
    /// Parent this into your layout.
    pub surround: Entity,
    /// The `TextInputNode`-owning inner Node. `TextInputContents` /
    /// `TextInputBuffer` / `TextInputQueue` / `TextInputFilter` /
    /// `TabIndex` all live here. Use this when you need to attach
    /// upstream components or focus the entity manually.
    pub inner: Entity,
}

impl TextInputHandle {
    /// Surround entity, for the common case where the caller just
    /// needs something to parent into their layout.
    #[inline]
    pub fn id(self) -> Entity {
        self.surround
    }
}

impl From<TextInputHandle> for Entity {
    fn from(h: TextInputHandle) -> Self {
        h.surround
    }
}

/// Spawn a styled single-line text input.
///
/// Layout is two entities: a **surround** that owns the border /
/// background / padding and centers the inner Node vertically via
/// flex, and an **inner** that owns the [`TextInputNode`]. cosmic-text
/// always anchors text to the top of its Node, so the inner is sized
/// to the line-height and the surround handles vertical centering.
///
/// Picking observers from `bevy_ui_text_input` (focus on click, drag,
/// multi-click selection, keyboard input dispatch) are attached
/// directly to the `TextInputNode` entity. Picking dispatches to the
/// topmost UI node under the cursor — the inner is rendered on top of
/// the surround (children render above parents in `bevy_ui`), so the
/// click reaches the inner's observers.
pub fn spawn_text_input(
    commands: &mut Commands,
    style: &TextInputStyle,
    initial: impl Into<String>,
    placeholder: impl Into<String>,
) -> TextInputHandle {
    spawn_text_input_inner(commands, style, initial, placeholder, None)
}

pub fn spawn_text_input_with_icon(
    commands: &mut Commands,
    style: &TextInputStyle,
    initial: impl Into<String>,
    placeholder: impl Into<String>,
    icon: Handle<Image>,
) -> TextInputHandle {
    spawn_text_input_inner(commands, style, initial, placeholder, Some(icon))
}

/// The leading icon's box.
///
/// Half the control's height, which is the same rule `spawn_button` applies to
/// a text-bearing button's icon (`height * 0.5`) and the same 16 the command
/// palette uses in an identical row. It was 14, which put the vertical gutter
/// at 9 — off the scale, and inconsistent with both neighbours.
const ICON_SIZE: f32 = 16.0;

fn spawn_text_input_inner(
    commands: &mut Commands,
    style: &TextInputStyle,
    initial: impl Into<String>,
    placeholder: impl Into<String>,
    icon: Option<Handle<Image>>,
) -> TextInputHandle {
    let initial = initial.into();
    let placeholder = placeholder.into();

    let text_font = style.font.text_font(style.typography.sm);
    let line_height = style.typography.sm * 1.2;

    let has_icon = icon.is_some();
    let surround = commands
        .spawn((
            TextInput,
            StyledNode::new()
                .height(ControlSize::Md)
                .radius(Step::new(1)),
            Node {
                width: Val::Px(DEFAULT_WIDTH),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::horizontal(Val::Px(10.0)),
                align_items: AlignItems::Center,
                column_gap: if has_icon {
                    Val::Px(style.spacing.sm)
                } else {
                    Val::ZERO
                },
                ..default()
            },
            BackgroundColor(style.palette.background),
            BorderColor::all(style.palette.input),
            Interaction::default(),
            crate::cursor::HoverCursor(bevy::window::SystemCursorIcon::Text),
            Name::new("tempera::text_input"),
        ))
        .id();

    if let Some(icon) = icon {
        commands.spawn((
            ImageNode::new(icon).with_color(style.palette.muted_foreground),
            Node {
                width: Val::Px(ICON_SIZE),
                height: Val::Px(ICON_SIZE),
                flex_shrink: 0.0,
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
            ChildOf(surround),
        ));
    }

    let mut inner = commands.spawn((
        TextInputNode {
            mode: TextInputMode::SingleLine,
            clear_on_submit: false,
            unfocus_on_submit: false,
            ..default()
        },
        // Opt-in to upstream's `update_text_input_contents` mirror —
        // without this component, the system silently skips the entity
        // and no String view of the buffer is ever populated. Tempera
        // treats `TextInputContents` as always-present, so insert it
        // here.
        TextInputContents::default(),
        text_font.clone(),
        TextColor(style.palette.foreground),
        UpstreamCursorStyle {
            cursor_color: style.palette.foreground,
            selection_color: style.palette.accent,
            selected_text_color: Some(style.palette.accent_foreground),
            cursor_width: 2.0,
            cursor_height: 1.0,
            blink_interval: 0.55,
            ..default()
        },
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(line_height),
            ..default()
        },
        // Stop the click-to-focus traversal here. Without TabIndex,
        // `bevy_input_focus::tab_navigation::acquire_focus` walks past
        // the text input, hits the Window, and clears focus — so the
        // input loses keyboard input the same frame it gains it.
        TabIndex(0),
        ChildOf(surround),
        Name::new("tempera::text_input::inner"),
    ));

    if !placeholder.is_empty() {
        inner.insert(TextInputPrompt {
            text: placeholder,
            font: Some(text_font),
            color: Some(style.palette.muted_foreground),
        });
    }

    let inner_id = inner.id();
    if !initial.is_empty() {
        commands.entity(inner_id).insert(TextInputQueue {
            actions: vec![TextInputAction::Edit(TextInputEdit::Paste(initial))].into(),
        });
    }

    TextInputHandle {
        surround,
        inner: inner_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeConfig;

    #[test]
    fn an_input_is_as_tall_as_the_declared_control() {
        // A text input, a default button and a select sit next to each
        // other in every form tempera renders. If their heights drift they
        // stop sharing a baseline — the exact defect that made the settings
        // dialog and the command palette look *almost* aligned. Each was
        // tuned separately; this is the assertion that says they are the
        // same quantity.
        let tokens = ThemeConfig::default().build().unwrap();
        assert_eq!(tokens.sizing.get(ControlSize::Md).get(), HEIGHT);
        assert_eq!(crate::button::ButtonSize::Md.height(), HEIGHT);
    }

    #[test]
    fn the_default_width_is_a_content_measure_and_stays_put() {
        // Deliberately *not* on the scale, and deliberately not moved by a
        // base change: how wide an input wants to be is a fact about the
        // text people type into it. Pinned so that a later sweep does not
        // "tidy" it onto the grid.
        assert_eq!(DEFAULT_WIDTH, 240.0);
    }
}
