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

use crate::theme::{ColorPalette, FontHandle, Step, StyledNode, ThemePlugin, Typography};

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
                    // Centre the glyph, because a cap is now usually wider
                    // than its content. Without this a `,` sits hard against
                    // the left padding of a box sized for a `W`.
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                // A square floor rather than a fixed size. Caps were sized
                // purely by content, so `,` and `I` came out 17px against
                // 21-24px for `B` and `F1` — a row of keys that visibly
                // failed to line up. `min_square` gives every cap the same
                // floor and lets `F1` or `Esc` still grow past it.
                StyledNode::new().min_square(Step::new(5)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(bevy::text::TextPlugin)
            .add_plugins(KbdPlugin);
        app.update();
        app
    }

    /// Spawn a chord and return its caps' `Node`s, in render order.
    fn caps(app: &mut App, chord: KbdChord) -> Vec<Node> {
        let world = app.world_mut();
        let mut state: SystemState<(Commands, KbdStyle)> = SystemState::new(world);
        let (mut commands, style) = state.get(world).expect("params validate");
        let row = spawn_kbd(&mut commands, &style, chord);
        state.apply(world);
        app.update();

        let world = app.world();
        world
            .get::<Children>(row)
            .expect("the row has caps")
            .iter()
            .map(|cap| world.get::<Node>(cap).expect("a cap has a Node").clone())
            .collect()
    }

    #[test]
    fn every_cap_shares_one_floor() {
        // The alignment property. Caps used to be sized purely by their
        // glyph, so a `,` cap measured 17px against 24px for `F1` — a row of
        // keys that visibly failed to line up. Asserting equality across
        // *different* glyphs is the point: a per-cap assertion would hold
        // even if each cap were sized independently.
        let mut app = app();
        let narrow = caps(&mut app, KbdChord::from(KeyCode::Comma));
        let wide = caps(&mut app, KbdChord::from(KeyCode::KeyW));

        assert_eq!(
            narrow[0].min_width, wide[0].min_width,
            "two caps disagree on their floor"
        );
        assert_ne!(
            narrow[0].min_width,
            Val::Auto,
            "no floor is set at all, so caps are still content-sized"
        );
    }

    #[test]
    fn a_cap_can_still_outgrow_the_floor() {
        // The half `square` would have broken. `Esc` and `F12` are wider
        // than a one-glyph cap, and a fixed width would clip them — so the
        // floor must be a *minimum*, never the width itself.
        let mut app = app();
        let cap = caps(&mut app, KbdChord::from(KeyCode::Escape));
        assert_eq!(
            cap[0].width,
            Val::Auto,
            "a fixed width would clip a long keycap"
        );
    }

    #[test]
    fn a_glyph_is_centred_in_its_cap() {
        // A cap is now usually wider than its glyph, so without this the
        // glyph sits against the left padding and the row reads as ragged
        // even though every box lines up.
        let mut app = app();
        let cap = caps(&mut app, KbdChord::from(KeyCode::Comma));
        assert_eq!(cap[0].justify_content, JustifyContent::Center);
        assert_eq!(cap[0].align_items, AlignItems::Center);
    }
}
