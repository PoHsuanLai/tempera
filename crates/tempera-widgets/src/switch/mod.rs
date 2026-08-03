//! Switch — toggle pill with sliding thumb.
//!
//! Shares headless behavior with [`crate::checkbox`]: built on
//! [`bevy::ui_widgets::Checkbox`] with the accessibility role
//! overridden to `Switch`. Visually distinct — a pill-shaped track
//! with a circular thumb that springs between left and right on
//! toggle (damped-spring animation via [`crate::anim::Spring`]).
//!
//! ## Composition
//!
//! - [`Checkbox`] (re-exported) — behavior marker
//! - [`Switch`] — tempera marker so paint systems target switches, not
//!   checkboxes
//! - [`SwitchThumb`] — child marker; paint system updates its `left`
//! - [`SwitchSize`] — sizing preset
//! - [`Checked`] — checked state
//! - [`InteractionDisabled`] — disabled state

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;

use crate::checkbox_behavior::CheckboxBehaviorPlugin;
use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use bevy::ui::Checked;
pub use bevy::ui_widgets::ValueChange;
pub use components::{Switch, SwitchSize, SwitchThumb};
pub use spawn::{SwitchStyle, spawn_switch, spawn_switch_sized};

pub struct SwitchStylePlugin;

impl Plugin for SwitchStylePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<CheckboxBehaviorPlugin>() {
            app.add_plugins(CheckboxBehaviorPlugin);
        }
        app.add_systems(
            Update,
            (
                systems::retarget_switch,
                systems::repaint_switch_track.run_if(
                    crate::theme::repaint_needed_on::<
                        Switch,
                        Or<(Changed<Checked>, Changed<InteractionDisabled>)>,
                    >,
                ),
                systems::drive_switch,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemePlugin;
    use bevy::ui::Checked;

    fn app() -> App {
        let mut app = App::new();
        // `Time` is inserted directly rather than via `TimePlugin`: the
        // plugin overwrites the generic clock from `Time<Virtual>` every
        // frame, which would clobber the manual `advance_by` the animation
        // test needs.
        app.init_resource::<Time>()
            .add_plugins(ThemePlugin)
            .add_systems(
                Update,
                (systems::retarget_switch, systems::drive_switch).chain(),
            );
        app
    }

    fn spawn(app: &mut App, checked: bool, size: SwitchSize) -> Entity {
        let mut state: bevy::ecs::system::SystemState<(Commands, SwitchStyle)> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let e = {
            let (mut commands, style) = state.get(app.world()).expect("theme present");
            spawn_switch_sized(&mut commands, &style, checked, size)
        };
        state.apply(app.world_mut());
        e
    }

    fn thumb_of(app: &App, root: Entity) -> Entity {
        app.world()
            .get::<Children>(root)
            .expect("a switch has a thumb")
            .iter()
            .find(|c| app.world().get::<SwitchThumb>(*c).is_some())
            .expect("thumb child")
    }

    fn px(v: Val) -> f32 {
        match v {
            Val::Px(v) => v,
            other => panic!("expected px, got {other:?}"),
        }
    }

    #[test]
    fn a_sized_switch_actually_gets_that_size() {
        // `spawn_switch` used to hardcode `SwitchSize::default()`, so the
        // component's other two variants were unreachable. A caller wanting a
        // small switch had to resize the thumb by hand afterwards — dawai
        // carried a system and a marker component to do exactly that.
        for size in [SwitchSize::Sm, SwitchSize::Md, SwitchSize::Lg] {
            let mut app = app();
            let e = spawn(&mut app, false, size);

            let node = app.world().get::<Node>(e).unwrap();
            assert_eq!(px(node.width), size.track_width(), "{size:?} track width");
            assert_eq!(
                px(node.height),
                size.track_height(),
                "{size:?} track height"
            );

            let thumb = app.world().get::<Node>(thumb_of(&app, e)).unwrap();
            assert_eq!(px(thumb.width), size.thumb_diameter(), "{size:?} thumb");
        }
    }

    #[test]
    fn the_default_spawn_still_matches_the_default_size() {
        let mut app = app();
        let plain = {
            let mut state: bevy::ecs::system::SystemState<(Commands, SwitchStyle)> =
                bevy::ecs::system::SystemState::new(app.world_mut());
            let e = {
                let (mut commands, style) = state.get(app.world()).expect("theme present");
                spawn_switch(&mut commands, &style, false)
            };
            state.apply(app.world_mut());
            e
        };
        let sized = spawn(&mut app, false, SwitchSize::default());

        let a = app.world().get::<Node>(plain).unwrap().width;
        let b = app.world().get::<Node>(sized).unwrap().width;
        assert_eq!(px(a), px(b));
    }

    #[test]
    fn a_toggled_thumb_lands_where_a_spawned_one_rests() {
        // The bug that consolidating `INSET` prevents, and it only appears
        // in *motion*: `spawn` placed the thumb with one formula and
        // `drive_switch` interpolated toward another. A switch spawned
        // checked never revealed it, because its spring starts settled at
        // 1.0 and the drive system returns early — so the two formulas had
        // to be compared by toggling a switch and letting it animate.
        for size in [SwitchSize::Sm, SwitchSize::Md, SwitchSize::Lg] {
            let mut app = app();
            let born_on = spawn(&mut app, true, size);
            let target = px(app
                .world()
                .get::<Node>(thumb_of(&app, born_on))
                .unwrap()
                .left);

            let toggled = spawn(&mut app, false, size);
            app.world_mut().entity_mut(toggled).insert(Checked);
            // `Time` does not advance on its own in a headless app, so the
            // spring would never move. Step it by hand.
            for _ in 0..600 {
                app.world_mut()
                    .resource_mut::<Time>()
                    .advance_by(std::time::Duration::from_millis(4));
                app.update();
            }
            let landed = px(app
                .world()
                .get::<Node>(thumb_of(&app, toggled))
                .unwrap()
                .left);

            assert!(
                (landed - target).abs() < 0.5,
                "{size:?}: a toggled thumb settled at {landed} but a spawned one rests at {target}"
            );
        }
    }

    #[test]
    fn an_off_switch_rests_against_the_near_edge() {
        let mut app = app();
        let off = spawn(&mut app, false, SwitchSize::Lg);
        assert_eq!(
            px(app.world().get::<Node>(thumb_of(&app, off)).unwrap().left),
            SwitchSize::INSET
        );
        assert!(app.world().get::<Checked>(off).is_none());
    }

    #[test]
    fn the_thumb_always_fits_inside_its_track() {
        // `thumb_diameter` is `track_height - 2 * INSET`, so this holds by
        // construction — but it is the invariant that makes `INSET` a
        // property of the size rather than a number two files happen to
        // share, and it should fail loudly if the derivation changes.
        for size in [SwitchSize::Sm, SwitchSize::Md, SwitchSize::Lg] {
            assert_eq!(
                size.thumb_diameter() + 2.0 * SwitchSize::INSET,
                size.track_height(),
                "{size:?} thumb does not fit its track"
            );
            assert!(size.thumb_travel() > 0.0, "{size:?} has nowhere to travel");
        }
    }
}
