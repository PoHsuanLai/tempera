//! Keyboard-shortcut chips — the drawn half.
//!
//! The chord *vocabulary* ([`KbdChord`], [`KbdKey`], the glyph tables) lives in
//! `tempera-input`, because `⌘` for Super is keyboard knowledge and nothing
//! about it depends on a palette or a font. This module is what turns one into
//! nodes: [`spawn_kbd`] for bordered chips, [`spawn_chord_inline`] for flat
//! glyph text inside a list row.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub use tempera_input::kbd::{KbdChord, KbdKey, key_glyph, modifier_glyph};

use crate::theme::{ColorPalette, FontHandle, ThemePlugin, Typography};

#[derive(SystemParam)]
pub struct KbdStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
}

/// Spawn an inline row of flat glyph-text segments for a [`KbdChord`].
/// Returns the row entity. Use this when the chord should read as part
/// of the surrounding list-row text (palette / context-menu items) —
/// for the bordered "kbd"-style chip render, use [`spawn_kbd`] instead.
pub fn spawn_chord_inline(
    commands: &mut Commands,
    parent: Entity,
    chord: &KbdChord,
    font: &FontHandle,
    typography: &Typography,
    color: Color,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(2.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(parent),
        ))
        .id();

    for segment in chord.render_order() {
        commands.spawn((
            // Marked here rather than by the caller: these are this
            // function's own entities, and a caller that wanted them
            // repainted would otherwise have to reach across the widget
            // boundary to tag children it did not spawn.
            KbdCapText,
            Text::new(segment.glyph()),
            font.text_font(typography.xs),
            TextColor(color),
            bevy::picking::Pickable::IGNORE,
            ChildOf(row),
        ));
    }

    row
}

/// Spawn a row of styled chips for a `KbdChord`. Returns the row
/// entity so the caller can re-parent it.
pub fn spawn_kbd(commands: &mut Commands, style: &KbdStyle, chord: impl Into<KbdChord>) -> Entity {
    let chord: KbdChord = chord.into();
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new("tempera::kbd"),
        ))
        .id();

    for segment in chord.render_order() {
        let display = segment.glyph();
        commands
            .spawn((
                Node {
                    padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                KbdCap,
                BackgroundColor(style.palette.muted),
                BorderColor::all(style.palette.border),
                ChildOf(row),
            ))
            .with_children(|parent| {
                parent.spawn((
                    KbdCapText,
                    Text::new(display),
                    style.font.text_font(style.typography.xs),
                    TextColor(style.palette.muted_foreground),
                    bevy::picking::Pickable::IGNORE,
                ));
            });
    }

    row
}

/// Marker on one keycap, so its fill and border can be repainted.
#[derive(Component, Default, Debug)]
pub struct KbdCap;

/// Marker on a keycap's glyph.
#[derive(Component, Default, Debug)]
pub struct KbdCapText;

/// Repaint the caps and their glyphs.
fn repaint_kbd(
    palette: Res<crate::theme::ColorPalette>,
    mut caps: Query<(&mut BackgroundColor, &mut BorderColor), With<KbdCap>>,
    mut glyphs: Query<&mut TextColor, With<KbdCapText>>,
) {
    let border_want = BorderColor::all(palette.border);
    for (mut bg, mut border) in &mut caps {
        if bg.0 != palette.muted {
            bg.0 = palette.muted;
        }
        if *border != border_want {
            *border = border_want;
        }
    }
    for mut color in &mut glyphs {
        if color.0 != palette.muted_foreground {
            color.0 = palette.muted_foreground;
        }
    }
}

pub struct KbdPlugin;

impl Plugin for KbdPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_systems(Update, repaint_kbd.run_if(crate::theme::palette_changed));
    }
}
