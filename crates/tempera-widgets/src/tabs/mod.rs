//! Tabs — horizontal segmented switcher with an animated indicator.
//!
//! Bevy 0.18's `bevy_ui_widgets` doesn't ship a Tabs primitive, so
//! tempera owns the behavior here. A tabs row is:
//!
//! - **Root** ([`Tabs`] + [`TabsActive`]) — a Node carrying the active
//!   index and emitting [`TabsChanged`] when it flips.
//! - **Triggers** ([`TabTrigger`]) — children with a click handler.
//! - **Indicator** ([`TabIndicator`]) — child Node lerped toward the
//!   active trigger's position by the paint system.
//!
//! ## Spawning
//!
//! ```ignore
//! let id = spawn_tabs(&mut commands, &style, vec!["Files", "Search", "Git"], 0);
//! commands.entity(id).observe(|on: On<TabsChanged>| info!("tab -> {}", on.active));
//! ```

use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui_widgets::ButtonPlugin as BevyButtonPlugin;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{TabIndicator, TabTrigger, Tabs, TabsActive};
pub use spawn::{TabsStyle, spawn_tabs};
pub use systems::TabsChanged;

pub struct TabsPlugin;

impl Plugin for TabsPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<InputDispatchPlugin>() {
            app.add_plugins(InputDispatchPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<BevyButtonPlugin>() {
            app.add_plugins(BevyButtonPlugin);
        }
        // TabsChanged is an EntityEvent triggered via `commands.trigger`;
        // no add_message registration needed.
        app.add_observer(systems::trigger_on_activate);
        app.add_systems(
            Update,
            (
                systems::repaint_triggers
                    .run_if(crate::theme::repaint_needed_on::<TabTrigger, Changed<TabsActive>>),
                systems::move_indicator,
            ),
        );
    }
}

#[cfg(test)]
mod styled_animation_tests {
    use super::*;
    use crate::theme::{StyledNode, ThemePlugin};

    /// A `StyledNode` on an animating widget must not fight its animation.
    ///
    /// This is the question that kept `slider`, `switch`, `tabs` and
    /// `tooltip` out of the first conversion pass. They write `left`, `top`,
    /// `width` and `border` every frame; `StyledNode` writes `padding`,
    /// gaps, `radius` and `height`. The sets look disjoint, but "looks
    /// disjoint" is exactly the kind of claim that should be a test — a
    /// later `.square()` on an animated part would silently start
    /// overwriting the animated `width` each time the theme moved.
    #[test]
    fn a_styled_tab_strip_leaves_the_indicator_alone() {
        let mut app = App::new();
        // Deliberately *without* `move_indicator`. The question is whether
        // `apply_styled_nodes` touches the animated fields, and running the
        // animation too would mask the answer — the indicator eases toward
        // its target every frame, so any assertion on `left` would move for
        // reasons that have nothing to do with the theme.
        app.add_plugins(ThemePlugin);

        // An indicator mid-animation, positioned by `move_indicator`.
        let strip = app
            .world_mut()
            .spawn((
                Tabs,
                TabsActive(0),
                StyledNode::new().height(crate::theme::ControlSize::Sm),
                Node::default(),
            ))
            .id();
        let indicator = app
            .world_mut()
            .spawn((
                TabIndicator,
                Node {
                    left: Val::Px(37.0),
                    width: Val::Px(64.0),
                    ..default()
                },
                ChildOf(strip),
            ))
            .id();
        app.update();

        // Move the theme: `apply_styled_nodes` runs over the strip.
        let coarse = crate::theme::ThemeConfig {
            density: crate::theme::Density::Spacious,
            ..default()
        };
        app.insert_resource(coarse)
            .insert_resource(coarse.build().expect("coherent"));
        app.update();

        let node = app.world().get::<Node>(indicator).unwrap();
        assert_eq!(
            (node.left, node.width),
            (Val::Px(37.0), Val::Px(64.0)),
            "a theme change moved fields the animation owns"
        );
        assert_eq!(
            app.world().get::<Node>(strip).unwrap().height,
            Val::Px(32.0),
            "the strip itself did follow the theme"
        );
    }
}
