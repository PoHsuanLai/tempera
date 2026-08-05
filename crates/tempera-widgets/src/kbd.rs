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

/// What a keycap is painted in.
///
/// The one thing a caller may vary. Geometry is *not* here on purpose:
/// a keycap is the same shape wherever it appears, and the surface it
/// sits on is the only reason two of them ever look different.
///
/// This exists because the tooltip had forked the whole renderer to change
/// three colours, and carried a private copy of the geometry along with them
/// — which then drifted (5/1 padding against 6/2) and did not pick up the
/// keycap alignment fix. Parameterising the colours makes the fork
/// unnecessary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KbdColors {
    pub fill: Color,
    pub border: Color,
    pub glyph: Color,
}

impl KbdColors {
    /// Caps on an ordinary surface: `muted` fill, `border`, muted glyph.
    #[must_use]
    pub fn standard(palette: &ColorPalette) -> Self {
        Self {
            fill: palette.muted,
            border: palette.border,
            glyph: palette.muted_foreground,
        }
    }

    /// Caps on an *inverted* surface — a tooltip popup, which paints
    /// `bg-foreground` / `text-background`.
    ///
    /// `standard` would put a `muted` fill on a near-`foreground` surface,
    /// where it reads as a smudge rather than a key. These sit lightly on
    /// top instead: a translucent tint of the popup's own text colour.
    #[must_use]
    pub fn on_inverted(palette: &ColorPalette) -> Self {
        let mut fill = palette.background;
        fill.set_alpha(0.18);
        let mut border = palette.background;
        border.set_alpha(0.32);
        Self {
            fill,
            border,
            glyph: palette.background,
        }
    }
}

/// Marker for caps whose colours the palette repaint must leave alone.
///
/// [`repaint_kbd`] resolves a cap back to [`KbdColors::standard`], which is
/// right for every cap on an ordinary surface and wrong for one on an
/// inverted popup — it would repaint a tooltip's caps to `muted` the first
/// time the theme changed. A tooltip despawns on hover-out and so cannot
/// outlive a palette swap, but that is a property of the tooltip rather than
/// of this module, and relying on it silently would break the moment
/// something else used these colours.
#[derive(Component, Default, Debug)]
pub struct KbdCustomColors;

/// Spawn a row of styled chips for a `KbdChord`. Returns the row
/// entity so the caller can re-parent it.
pub fn spawn_kbd(commands: &mut Commands, style: &KbdStyle, chord: impl Into<KbdChord>) -> Entity {
    spawn_kbd_in(
        commands,
        chord,
        KbdColors::standard(&style.palette),
        Repaint::FollowsPalette,
        &style.font,
        &style.typography,
    )
}

/// Whether the palette repaint owns a cap's colours, or the caller does.
///
/// Stated by the caller rather than inferred from the colours: two palettes
/// can coincide, so comparing values would silently hand ownership back to
/// the repaint the moment they did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repaint {
    /// [`repaint_kbd`] keeps these caps on the current palette.
    FollowsPalette,
    /// The caller owns them; the repaint leaves them alone.
    CallerOwns,
}

/// [`spawn_kbd`] with the colours chosen by the caller.
///
/// Same geometry, same markers, same alignment floor — only the paint
/// differs. Takes the pieces of [`KbdStyle`] it needs rather than the whole
/// param, so a caller that already holds a palette for its own surface (the
/// tooltip does) is not forced to acquire a second one.
pub fn spawn_kbd_in(
    commands: &mut Commands,
    chord: impl Into<KbdChord>,
    colors: KbdColors,
    repaint: Repaint,
    font: &FontHandle,
    typography: &Typography,
) -> Entity {
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
        let mut cap = commands.spawn((
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
            BackgroundColor(colors.fill),
            BorderColor::all(colors.border),
            ChildOf(row),
        ));
        if repaint == Repaint::CallerOwns {
            cap.insert(KbdCustomColors);
        }
        cap.with_children(|parent| {
            let mut glyph = parent.spawn((
                KbdCapText,
                Text::new(display),
                font.text_font(typography.xs),
                TextColor(colors.glyph),
                bevy::picking::Pickable::IGNORE,
            ));
            // Marked on the glyph as well as the cap. `repaint_kbd` queries
            // the two separately — a cap has the fill, its child has the
            // text — so a filter on the parent alone would still recolour
            // the glyph, leaving a caller-owned cap half repainted.
            if repaint == Repaint::CallerOwns {
                glyph.insert(KbdCustomColors);
            }
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

/// Caps the palette owns — everything not painted by a caller.
type ThemedCaps<'w, 's> = Query<
    'w,
    's,
    (&'static mut BackgroundColor, &'static mut BorderColor),
    (With<KbdCap>, Without<KbdCustomColors>),
>;

/// Their glyphs. Filtered on the glyph's own entity rather than its cap's,
/// because a `Without` on the parent does not reach a child.
type ThemedGlyphs<'w, 's> =
    Query<'w, 's, &'static mut TextColor, (With<KbdCapText>, Without<KbdCustomColors>)>;

/// Repaint the caps and their glyphs.
///
/// Skips anything marked [`KbdCustomColors`]: those were painted for a
/// surface this system knows nothing about, and resolving them to the
/// standard palette would be a downgrade rather than a refresh.
fn repaint_kbd(
    palette: Res<crate::theme::ColorPalette>,
    mut caps: ThemedCaps,
    mut glyphs: ThemedGlyphs,
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

#[cfg(test)]
mod shared_style_tests {
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

    /// Spawn a chord with the given colours; return the row.
    fn spawn(app: &mut App, colors_of: fn(&ColorPalette) -> KbdColors, repaint: Repaint) -> Entity {
        let world = app.world_mut();
        let mut state: SystemState<(Commands, KbdStyle)> = SystemState::new(world);
        let (mut commands, style) = state.get(world).expect("params validate");
        let colors = colors_of(&style.palette);
        let font = style.font.clone();
        let typography = style.typography.clone();
        let row = spawn_kbd_in(
            &mut commands,
            KbdChord::from(KeyCode::Comma),
            colors,
            repaint,
            &font,
            &typography,
        );
        state.apply(world);
        app.update();
        row
    }

    fn first_cap(app: &App, row: Entity) -> Entity {
        app.world().get::<Children>(row).expect("caps")[0]
    }

    #[test]
    fn an_inverted_cap_has_the_same_geometry_as_a_standard_one() {
        // The point of the refactor. The tooltip forked `spawn_kbd` to change
        // three colours and took a private copy of the geometry with it,
        // which then drifted to 5/1 padding and never gained the alignment
        // floor. Sharing the spawner is what stops that recurring — so the
        // assertion is that *only* colour differs.
        let mut app = app();
        let standard = spawn(&mut app, KbdColors::standard, Repaint::FollowsPalette);
        let inverted = spawn(&mut app, KbdColors::on_inverted, Repaint::CallerOwns);

        let a = app.world().get::<Node>(first_cap(&app, standard)).unwrap();
        let b = app.world().get::<Node>(first_cap(&app, inverted)).unwrap();

        assert_eq!(a.padding, b.padding, "the two caps disagree on padding");
        assert_eq!(a.border, b.border, "the two caps disagree on their border");
        assert_eq!(
            a.min_width, b.min_width,
            "an inverted cap missed the alignment floor"
        );
        assert_eq!(a.border_radius, b.border_radius);
    }

    #[test]
    fn an_inverted_cap_is_painted_differently() {
        // The other half: shared geometry must not mean shared paint, or the
        // tooltip's caps go back to reading as a smudge on its inverted
        // surface — which is what the fork existed to avoid.
        let mut app = app();
        let standard = spawn(&mut app, KbdColors::standard, Repaint::FollowsPalette);
        let inverted = spawn(&mut app, KbdColors::on_inverted, Repaint::CallerOwns);

        let a = app
            .world()
            .get::<BackgroundColor>(first_cap(&app, standard))
            .unwrap()
            .0;
        let b = app
            .world()
            .get::<BackgroundColor>(first_cap(&app, inverted))
            .unwrap()
            .0;
        assert_ne!(a, b, "the inverted variant is not actually inverted");
    }

    #[test]
    fn the_repaint_leaves_caller_owned_caps_alone() {
        // `repaint_kbd` resolves a cap to the standard palette. Run over a
        // tooltip's caps it would repaint them to `muted` — correct-looking
        // in isolation, wrong on an inverted popup.
        //
        // A tooltip despawns on hover-out and so cannot in practice outlive a
        // palette swap, which is exactly why this needs a test rather than a
        // screenshot: the bug is unreachable through the UI today and would
        // land the moment anything else used these colours.
        let mut app = app();
        let row = spawn(&mut app, KbdColors::on_inverted, Repaint::CallerOwns);
        let cap = first_cap(&app, row);
        let before = app.world().get::<BackgroundColor>(cap).unwrap().0;

        app.world_mut().insert_resource(ColorPalette::light());
        app.update();
        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(cap).unwrap().0,
            before,
            "the palette repaint clobbered a caller-owned cap"
        );
    }

    #[test]
    fn a_standard_cap_still_follows_the_palette() {
        // The converse, so the opt-out above cannot be implemented by simply
        // disabling the repaint for everyone.
        let mut app = app();
        let row = spawn(&mut app, KbdColors::standard, Repaint::FollowsPalette);
        let cap = first_cap(&app, row);

        app.world_mut().insert_resource(ColorPalette::light());
        app.update();
        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(cap).unwrap().0,
            ColorPalette::light().muted,
            "a standard cap stopped following the theme"
        );
    }
}
