use bevy::ecs::system::SystemParam;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy_resvg::prelude::{SvgColor, SvgFile, UiSvg};

use super::components::{
    Dialog, DialogBackdrop, DialogCard, DialogClose, DialogContent, DialogTitle, DialogTitleBar,
};
use super::messages::DialogDismissed;
use super::systems::Z_DIALOG;
use crate::theme::{ColorPalette, ControlSize, FontHandle, Metrics, Step, Typography};

/// Default card width / height — sized for a settings-style modal.
pub const CARD_WIDTH: f32 = 720.0;
pub const CARD_HEIGHT: f32 = 480.0;
/// The card's corner radius.
///
/// Step 3 on the spacing scale — 12 at the default base. Kept `pub` because
/// downstream code positions things against the card's corner; the value now
/// comes from the scale rather than from this literal.
pub const CARD_RADIUS: f32 = 12.0;
pub const CARD_PADDING: f32 = 0.0;
/// Title bar height.
///
/// `control_lg` — the declared large control height, which is what a title bar
/// is. This was a bare 44.0 and got hand-copied downstream as one.
pub const TITLE_BAR_HEIGHT: f32 = 44.0;
/// Horizontal padding inside the title bar.
///
/// Step 4 — 16 at the default base. Was 18, which is off the scale and, more
/// to the point, bears no relation to the 12px card radius it sits inside. At
/// 16 the gap between the card's corner and the bar's content is 4, which is
/// `Radius::concentric_inner(12, 4)` — the one relationship in the theme with
/// a proof behind it.
pub const TITLE_BAR_PADDING_X: f32 = 16.0;

/// The slice of theme tokens read by dialog spawn.
#[derive(SystemParam)]
pub struct DialogStyle<'w> {
    pub palette: Res<'w, ColorPalette>,
    pub metrics: Metrics<'w>,
    pub typography: Res<'w, Typography>,
    pub font: Res<'w, FontHandle>,
}

/// Configuration knobs for a spawned dialog.
///
/// When `closable` is true, the caller **must** supply a `close_icon`
/// — there is no text fallback. The dialog crate doesn't ship its own
/// icon assets; consumers wire their own raster registry (SVG → image
/// handle).
#[derive(Clone, Debug)]
pub struct DialogConfig {
    pub title: Option<String>,
    pub width: f32,
    pub height: f32,
    pub closable: bool,
    pub close_icon: Option<Handle<SvgFile>>,
    /// Start visible (`Visibility::Inherited`) or hidden
    /// (`Visibility::Hidden`). Default: hidden — toggle to inherited
    /// when ready to show.
    pub start_visible: bool,
}

impl Default for DialogConfig {
    fn default() -> Self {
        Self {
            title: None,
            width: CARD_WIDTH,
            height: CARD_HEIGHT,
            closable: false,
            close_icon: None,
            start_visible: false,
        }
    }
}

impl DialogConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    #[must_use]
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.width = w;
        self.height = h;
        self
    }

    /// Enable the close button, supplying the icon to render inside it.
    /// Calling this implies `closable = true`.
    #[must_use]
    pub fn closable_with_icon(mut self, icon: Handle<SvgFile>) -> Self {
        self.closable = true;
        self.close_icon = Some(icon);
        self
    }

    #[must_use]
    pub fn start_visible(mut self, v: bool) -> Self {
        self.start_visible = v;
        self
    }
}

/// Entities returned by [`spawn_dialog`]. Parent your custom widgets
/// to `content`. Toggle `root.Visibility` to show / hide.
pub struct DialogParts {
    pub root: Entity,
    pub backdrop: Entity,
    pub card: Entity,
    /// `Some` only when [`DialogConfig::closable`] is true.
    pub close_button: Option<Entity>,
    /// The flex column inside the card below the title bar. Parent
    /// user content here.
    pub content: Entity,
}

/// Spawn a dialog node tree:
///
/// ```text
/// root (full-screen, GlobalZIndex=Z_DIALOG, picks events)
///   ├── backdrop (full-screen, translucent, click → DialogDismissed)
///   └── card (centered, fixed size, BackgroundColor=palette.card)
///         ├── title row (title text + optional close button)
///         └── content (flex column, scrolls — user parents here)
/// ```
///
/// The dialog **does not auto-hide** on dismissal; consumers react to
/// [`DialogDismissed`] and flip `Visibility` themselves. This keeps
/// dialogs source-of-truth-agnostic (the open flag can live anywhere).
pub fn spawn_dialog(
    commands: &mut Commands,
    style: &DialogStyle,
    cfg: DialogConfig,
) -> DialogParts {
    let visibility = if cfg.start_visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    let root = commands
        .spawn((
            Dialog,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            visibility,
            GlobalZIndex(Z_DIALOG),
            Name::new("tempera::dialog"),
        ))
        .id();

    let backdrop = commands
        .spawn((
            DialogBackdrop,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(style.palette.scrim),
            ChildOf(root),
            Name::new("tempera::dialog::backdrop"),
        ))
        .observe(
            move |on: On<Pointer<Click>>,
                  mut writer: MessageWriter<DialogDismissed>,
                  card_q: Query<&ChildOf, With<DialogCard>>,
                  parent_q: Query<&ChildOf, With<DialogBackdrop>>| {
                // Only fire when the click landed on the backdrop
                // itself, not bubbled up from the card.
                if let Ok(parent) = parent_q.get(on.entity) {
                    let _ = card_q;
                    writer.write(DialogDismissed { dialog: parent.0 });
                }
            },
        )
        .id();

    let card = commands
        .spawn((
            DialogCard,
            Node {
                position_type: PositionType::Relative,
                width: Val::Px(cfg.width),
                height: Val::Px(cfg.height),
                flex_direction: FlexDirection::Column,
                border_radius: BorderRadius::all(style.metrics.radius(Step::new(3)).into()),
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(style.palette.card),
            BorderColor::all(style.palette.border),
            ChildOf(root),
            Name::new("tempera::dialog::card"),
        ))
        .id();

    // Title bar — only if a title is set or the dialog is closable.
    let mut close_button: Option<Entity> = None;
    if cfg.title.is_some() || cfg.closable {
        let title_bar = commands
            .spawn((
                DialogTitleBar,
                Node {
                    width: Val::Percent(100.0),
                    height: style.metrics.control(ControlSize::Lg).into(),
                    padding: UiRect::axes(style.metrics.gap(Step::new(4)).into(), Val::Px(0.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(style.palette.border),
                ChildOf(card),
            ))
            .id();

        if let Some(title) = &cfg.title {
            commands.spawn((
                DialogTitle,
                Text::new(title.clone()),
                style.font.text_font(style.typography.base),
                TextColor(style.palette.foreground),
                ChildOf(title_bar),
            ));
        } else {
            // Spacer so the close button stays right-aligned.
            commands.spawn((Node::default(), ChildOf(title_bar)));
        }

        if let (true, Some(icon)) = (cfg.closable, cfg.close_icon.clone()) {
            let btn = commands
                .spawn((
                    DialogClose,
                    Button,
                    // Step 0 — 4 at the default base. *Not* the card's
                    // radius run through `concentric_inner`, which is what
                    // the design doc names as the example for that rule.
                    // Checked, and it does not apply: the button is centred
                    // in the title bar, not nested in the card's corner, so
                    // there is no offset relating the two. Feeding it the
                    // title padding gives `max(0, 12 - 16)` = a square
                    // button, and the vertical inset gives 1. It is simply a
                    // small radius on a small button.
                    Node {
                        width: Val::Px(22.0),
                        height: Val::Px(22.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(style.metrics.radius(Step::BASE).into()),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ChildOf(title_bar),
                ))
                .observe(
                    move |on: On<Pointer<Click>>,
                          parent_q: Query<&ChildOf, With<DialogClose>>,
                          title_parent_q: Query<&ChildOf>,
                          mut writer: MessageWriter<DialogDismissed>| {
                        // close button -> title bar -> card -> root
                        let Ok(p1) = parent_q.get(on.entity) else {
                            return;
                        };
                        let Ok(p2) = title_parent_q.get(p1.0) else {
                            return;
                        };
                        let Ok(p3) = title_parent_q.get(p2.0) else {
                            return;
                        };
                        writer.write(DialogDismissed { dialog: p3.0 });
                    },
                )
                .id();
            commands.spawn((
                UiSvg(icon),
                SvgColor(style.palette.muted_foreground),
                Node {
                    width: Val::Px(12.0),
                    height: Val::Px(12.0),
                    ..default()
                },
                ChildOf(btn),
            ));
            close_button = Some(btn);
        }
    }

    let content = commands
        .spawn((
            DialogContent,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip_y(),
                ..default()
            },
            ChildOf(card),
            Name::new("tempera::dialog::content"),
        ))
        .id();

    DialogParts {
        root,
        backdrop,
        card,
        close_button,
        content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemePlugin;
    use bevy::ecs::system::SystemState;

    /// The backdrop reads the palette's scrim rather than a colour of its own.
    ///
    /// The first dialog test. Without it the token could be correct and
    /// unused: reverting the call site to the hardcoded black it replaced
    /// would leave every other test in the workspace green, because nothing
    /// else looks at what a backdrop is painted.
    ///
    /// Asserted against the *light* palette specifically. The dark value is
    /// the one the call site used to hardcode, so a stale literal would still
    /// match there — light is where a hardcoded colour and the token disagree.
    #[test]
    fn the_backdrop_is_painted_from_the_palette() {
        let mut app = App::new();
        app.add_plugins(ThemePlugin)
            .insert_resource(ColorPalette::light());

        let world = app.world_mut();
        let mut state: SystemState<(Commands, DialogStyle)> = SystemState::new(world);
        let backdrop = {
            let (mut commands, style) = state.get(world).expect("theme resources present");
            spawn_dialog(&mut commands, &style, DialogConfig::default()).backdrop
        };
        state.apply(world);

        let painted = world.get::<BackgroundColor>(backdrop).unwrap().0;
        let want = world.resource::<ColorPalette>().scrim;
        assert_eq!(
            painted.to_srgba(),
            want.to_srgba(),
            "the backdrop is not reading ColorPalette::scrim"
        );
    }

    /// Every dialog surface follows a theme change.
    ///
    /// The bug this covers: all five values below are written once at spawn,
    /// so a dialog that was open when the user switched theme stayed in the
    /// old one for as long as it lived. For a *settings* dialog that is the
    /// entire time they are choosing a theme — the one dialog where the fault
    /// is guaranteed to be seen.
    ///
    /// Driven through `App::update` rather than by calling the system, so the
    /// run condition is under test too. A repaint that works only when called
    /// by hand is the same bug in a new place.
    #[test]
    fn a_palette_swap_repaints_every_dialog_surface() {
        use super::super::{DialogPlugin, DialogTitle, DialogTitleBar};

        let mut app = App::new();
        // `DialogPlugin` brings `SvgPlugin`, whose loader needs `Assets<_>`.
        // Added here rather than reaching for a bare `ThemePlugin`, because
        // the run condition that gates the repaint is part of what is under
        // test and only the real plugin registers it.
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(bevy::image::ImagePlugin::default())
            // `dismiss_on_escape` reads `KeyboardInput`, and an unregistered
            // message fails system-param validation outright rather than
            // reading empty.
            .add_plugins(bevy::input::InputPlugin)
            .add_plugins(DialogPlugin)
            .insert_resource(ColorPalette::dark());

        let world = app.world_mut();
        let mut state: SystemState<(Commands, DialogStyle)> = SystemState::new(world);
        let parts = {
            let (mut commands, style) = state.get(world).expect("theme resources present");
            spawn_dialog(
                &mut commands,
                &style,
                DialogConfig {
                    title: Some("Settings".into()),
                    closable: true,
                    ..default()
                },
            )
        };
        state.apply(world);
        app.update();

        // Find the two nodes that have no handle on `DialogParts`.
        let title_bar = app
            .world_mut()
            .query_filtered::<Entity, With<DialogTitleBar>>()
            .single(app.world())
            .expect("a titled dialog has a title bar");
        let title = app
            .world_mut()
            .query_filtered::<Entity, With<DialogTitle>>()
            .single(app.world())
            .expect("a titled dialog has a title");

        app.world_mut().insert_resource(ColorPalette::light());
        app.update();

        let light = ColorPalette::light();
        let w = app.world();
        assert_eq!(
            w.get::<BackgroundColor>(parts.backdrop).unwrap().0,
            light.scrim,
            "the backdrop kept the old theme"
        );
        assert_eq!(
            w.get::<BackgroundColor>(parts.card).unwrap().0,
            light.card,
            "the card kept the old theme"
        );
        assert_eq!(
            *w.get::<BorderColor>(parts.card).unwrap(),
            BorderColor::all(light.border),
            "the card border kept the old theme"
        );
        assert_eq!(
            *w.get::<BorderColor>(title_bar).unwrap(),
            BorderColor::all(light.border),
            "the title rule kept the old theme"
        );
        assert_eq!(
            w.get::<TextColor>(title).unwrap().0,
            light.foreground,
            "the title text kept the old theme"
        );
    }

    /// An untitled dialog has no title row, and repainting must not assume one.
    ///
    /// `title` and `closable` are both optional, so the two marked nodes may
    /// not exist. A `single()` in the system rather than a loop would panic
    /// here — on a dialog shape the crate explicitly supports.
    #[test]
    fn repainting_an_untitled_dialog_touches_only_what_exists() {
        use super::super::DialogPlugin;

        let mut app = App::new();
        // `DialogPlugin` brings `SvgPlugin`, whose loader needs `Assets<_>`.
        // Added here rather than reaching for a bare `ThemePlugin`, because
        // the run condition that gates the repaint is part of what is under
        // test and only the real plugin registers it.
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(bevy::image::ImagePlugin::default())
            // `dismiss_on_escape` reads `KeyboardInput`, and an unregistered
            // message fails system-param validation outright rather than
            // reading empty.
            .add_plugins(bevy::input::InputPlugin)
            .add_plugins(DialogPlugin)
            .insert_resource(ColorPalette::dark());

        let world = app.world_mut();
        let mut state: SystemState<(Commands, DialogStyle)> = SystemState::new(world);
        let parts = {
            let (mut commands, style) = state.get(world).expect("theme resources present");
            spawn_dialog(
                &mut commands,
                &style,
                DialogConfig {
                    title: None,
                    closable: false,
                    ..default()
                },
            )
        };
        state.apply(world);
        app.update();

        app.world_mut().insert_resource(ColorPalette::light());
        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(parts.card).unwrap().0,
            ColorPalette::light().card,
            "an untitled dialog did not repaint"
        );
    }
}
