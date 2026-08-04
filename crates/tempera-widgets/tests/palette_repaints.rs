//! Every widget follows a theme change, including its text.
//!
//! One suite rather than a test per module, because the property is identical
//! across all of them and the interesting failure is *which widget was
//! forgotten* — a question a single table answers and six scattered tests do
//! not.
//!
//! # The bug this exists for
//!
//! A colour written once at spawn cannot follow a palette swap. Eight widgets
//! already gated a repaint on `palette_changed`; these six did not, and the
//! gap was invisible for a long time for one specific reason: most static text
//! is `muted_foreground`, which is mid-grey in both palettes and legible
//! either way. Only `foreground` actually *inverts* — so the first thing to
//! expose it was a settings row in a light theme, where the label went
//! white-on-white and the widget looked empty rather than mistinted.
//!
//! That is why the assertions below deliberately include the `foreground`
//! cases (setting-row label, select display, card title) rather than sampling
//! one colour per widget.

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use tempera::theme::{ColorPalette, ThemePlugin};

/// A headless app with the theme resources and a font, starting dark.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .add_plugins(bevy::image::ImagePlugin::default())
        .add_plugins(bevy::text::TextPlugin)
        .add_plugins(ThemePlugin)
        .insert_resource(ColorPalette::dark());
    app
}

/// Swap to light and let the repaint systems run.
fn go_light(app: &mut App) {
    app.update();
    app.world_mut().insert_resource(ColorPalette::light());
    app.update();
}

#[test]
fn a_setting_rows_text_follows_the_theme() {
    use tempera::setting_row::{
        SettingRowLabel, SettingRowPlugin, SettingRowSpec, SettingRowStyle, SettingSectionLabel,
        spawn_section_header, spawn_setting_row,
    };

    let mut app = app();
    app.add_plugins(SettingRowPlugin);

    let world = app.world_mut();
    let mut state: SystemState<(Commands, SettingRowStyle)> = SystemState::new(world);
    {
        let (mut commands, style) = state.get(world).expect("theme resources present");
        let root = commands.spawn(Node::default()).id();
        spawn_section_header(&mut commands, &style, root, "THEME");
        spawn_setting_row(
            &mut commands,
            &style,
            root,
            SettingRowSpec::new("Colour theme").description("Applies immediately"),
        );
    }
    state.apply(world);

    go_light(&mut app);

    let light = ColorPalette::light();
    let label = app
        .world_mut()
        .query_filtered::<&TextColor, With<SettingRowLabel>>()
        .single(app.world())
        .expect("the row has a label")
        .0;
    // The one that inverts, and so the one that actually disappeared.
    assert_eq!(label, light.foreground, "the row label kept the old theme");

    let section = app
        .world_mut()
        .query_filtered::<&TextColor, With<SettingSectionLabel>>()
        .single(app.world())
        .expect("the header has a label")
        .0;
    assert_eq!(
        section, light.muted_foreground,
        "the section heading kept the old theme"
    );
}

#[test]
fn a_selects_display_text_follows_the_theme() {
    use tempera::select::{
        SelectDisplayText, SelectOption, SelectPlugin, SelectStyle, spawn_select,
    };

    let mut app = app();
    // A select opens its options through the context menu, so its systems read
    // `MenuItemActivated` — an unregistered message fails system-param
    // validation rather than reading empty. The menu plugin owns it.
    app.add_plugins(bevy::input::InputPlugin)
        .add_plugins(bevy::picking::PickingPlugin)
        .add_plugins(bevy::picking::InteractionPlugin)
        .add_plugins(tempera::context_menu::ContextMenuPlugin)
        .add_plugins(SelectPlugin);

    let world = app.world_mut();
    let mut state: SystemState<(Commands, SelectStyle)> = SystemState::new(world);
    {
        let (mut commands, style) = state.get(world).expect("theme resources present");
        spawn_select(
            &mut commands,
            &style,
            vec![SelectOption {
                id: "dark".into(),
                label: "Dark".into(),
            }],
            "dark",
        );
    }
    state.apply(world);

    go_light(&mut app);

    let display = app
        .world_mut()
        .query_filtered::<&TextColor, With<SelectDisplayText>>()
        .single(app.world())
        .expect("the select shows its value")
        .0;
    assert_eq!(
        display,
        ColorPalette::light().foreground,
        "the select's value went white-on-white"
    );
}

#[test]
fn a_cards_surface_and_title_follow_the_theme() {
    use tempera::card::{Card, CardPlugin, CardStyle, CardTitle, spawn_card};

    let mut app = app();
    app.add_plugins(CardPlugin);

    let world = app.world_mut();
    let mut state: SystemState<(Commands, CardStyle)> = SystemState::new(world);
    {
        let (mut commands, style) = state.get(world).expect("theme resources present");
        let root = commands.spawn(Node::default()).id();
        spawn_card(
            &mut commands,
            &style,
            root,
            "Details",
            tempera::card::CardState::Expanded,
        );
    }
    state.apply(world);

    go_light(&mut app);

    let light = ColorPalette::light();
    let fill = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<Card>>()
        .single(app.world())
        .expect("one card")
        .0;
    assert_eq!(fill, light.card, "the card fill kept the old theme");

    let title = app
        .world_mut()
        .query_filtered::<&TextColor, With<CardTitle>>()
        .single(app.world())
        .expect("the card has a title")
        .0;
    assert_eq!(title, light.foreground, "the card title kept the old theme");
}

#[test]
fn a_keycap_follows_the_theme() {
    use tempera::kbd::{KbdCap, KbdCapText, KbdPlugin, KbdStyle, spawn_kbd};

    let mut app = app();
    app.add_plugins(KbdPlugin);

    let world = app.world_mut();
    let mut state: SystemState<(Commands, KbdStyle)> = SystemState::new(world);
    {
        let (mut commands, style) = state.get(world).expect("theme resources present");
        spawn_kbd(&mut commands, &style, KeyCode::F2);
    }
    state.apply(world);

    go_light(&mut app);

    let light = ColorPalette::light();
    let cap = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<KbdCap>>()
        .iter(app.world())
        .next()
        .expect("at least one cap")
        .0;
    assert_eq!(cap, light.muted, "the keycap kept the old theme");

    let glyph = app
        .world_mut()
        .query_filtered::<&TextColor, With<KbdCapText>>()
        .iter(app.world())
        .next()
        .expect("a cap has a glyph")
        .0;
    assert_eq!(
        glyph, light.muted_foreground,
        "the keycap glyph kept the old theme"
    );
}

#[test]
fn a_separator_follows_the_theme() {
    use tempera::separator::{
        Separator, SeparatorAxis, SeparatorPlugin, SeparatorStyle, spawn_separator,
    };

    let mut app = app();
    app.add_plugins(SeparatorPlugin);

    let world = app.world_mut();
    let mut state: SystemState<(Commands, SeparatorStyle)> = SystemState::new(world);
    {
        let (mut commands, style) = state.get(world).expect("theme resources present");
        spawn_separator(&mut commands, &style, SeparatorAxis::Horizontal, None);
    }
    state.apply(world);

    go_light(&mut app);

    let rule = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<Separator>>()
        .single(app.world())
        .expect("one separator")
        .0;
    assert_eq!(
        rule,
        ColorPalette::light().border,
        "the rule kept the old theme"
    );
}

#[test]
fn a_progress_bar_follows_the_theme() {
    use tempera::progress::{
        Progress, ProgressFill, ProgressPlugin, ProgressStyle, spawn_progress,
    };

    let mut app = app();
    app.add_plugins(ProgressPlugin);

    let world = app.world_mut();
    let mut state: SystemState<(Commands, ProgressStyle)> = SystemState::new(world);
    {
        let (mut commands, style) = state.get(world).expect("theme resources present");
        spawn_progress(&mut commands, &style, 200.0, 0.5);
    }
    state.apply(world);

    go_light(&mut app);

    let light = ColorPalette::light();
    let track = app
        .world_mut()
        .query_filtered::<&BackgroundColor, (With<Progress>, Without<ProgressFill>)>()
        .single(app.world())
        .expect("one track")
        .0;
    assert_eq!(track, light.muted, "the track kept the old theme");

    let fill = app
        .world_mut()
        .query_filtered::<&BackgroundColor, With<ProgressFill>>()
        .single(app.world())
        .expect("one fill")
        .0;
    assert_eq!(fill, light.primary, "the fill kept the old theme");
}

/// A tooltip is *not* in this suite, and that is a finding rather than an
/// omission.
///
/// It paints five values from the palette at spawn and has no repaint system,
/// which looks identical to the six bugs above. It is not one: the popup is
/// despawned on hover-out and respawned on hover-in, so it never outlives a
/// theme change and always reads a current palette.
///
/// Recorded here rather than left as a gap someone re-derives later — and as
/// an assertion rather than a comment, so that if the lifetime ever changes to
/// a persistent popup this stops being true out loud.
#[test]
fn a_tooltip_popup_does_not_outlive_a_hover() {
    use tempera::tooltip::TooltipPopup;

    let mut app = app();
    app.add_plugins(tempera::tooltip::TooltipPlugin);
    app.update();

    let popups = app
        .world_mut()
        .query_filtered::<Entity, With<TooltipPopup>>()
        .iter(app.world())
        .count();
    assert_eq!(
        popups, 0,
        "a tooltip exists with nothing hovered, so it can outlive a theme \
         change and needs a repaint system like the widgets above"
    );
}
