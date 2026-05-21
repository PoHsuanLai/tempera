use bevy::ecs::system::SystemParam;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy_ui_text_input::actions::{TextInputAction, TextInputEdit};
use bevy_ui_text_input::{
    TextInputContents, TextInputMode, TextInputNode, TextInputPrompt, TextInputQueue,
    TextInputStyle as UpstreamCursorStyle,
};

use super::components::TextInput;
use crate::theme::{ColorPalette, FontHandle, Spacing, Typography};

pub(crate) const HEIGHT: f32 = 32.0;
pub(crate) const DEFAULT_WIDTH: f32 = 240.0;

#[derive(SystemParam)]
pub struct TextInputStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub spacing: Res<'w, Spacing>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
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
    let initial = initial.into();
    let placeholder = placeholder.into();

    let text_font = style.font.text_font(style.typography.sm);
    // `LineHeight::RelativeToFont(1.2)` is the default applied by the
    // upstream `TextInputNode` require chain. Mirror that here so the
    // inner Node sizes itself exactly to the rendered text.
    let line_height = style.typography.sm * 1.2;

    let surround = commands
        .spawn((
            TextInput,
            Node {
                width: Val::Px(DEFAULT_WIDTH),
                height: Val::Px(HEIGHT),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(style.spacing.corner_radius_small)),
                padding: UiRect::horizontal(Val::Px(10.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(style.palette.background),
            BorderColor::all(style.palette.input),
            Interaction::default(),
            crate::cursor::HoverCursor(bevy::window::SystemCursorIcon::Text),
            Name::new("tempera::text_input"),
        ))
        .id();

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
