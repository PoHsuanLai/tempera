//! Toast — non-modal notifications, entity-native.
//!
//! Each toast is an entity. Spawn with [`spawn`], [`spawn_error`], or the
//! [`ToastSpec`] builder. The lifecycle systems pick up new entities each
//! frame, build their UI subtrees, advance the slide spring, and auto-dismiss
//! when the countdown elapses — a caller that only wants to say something need
//! not keep the returned [`Entity`] at all.
//!
//! ```ignore
//! use tempera::toast;
//!
//! fn save(mut commands: Commands) {
//!     toast::spawn(&mut commands, "Project saved");
//!     toast::spawn_error(&mut commands, "Save failed");
//! }
//! ```
//!
//! Toasts slide in from the configured corner (default
//! [`ToastPosition::BottomRight`]), show a variant-colored accent dot, and
//! auto-dismiss after their [`ToastDuration`].
//!
//! # Long-running work
//!
//! Report progress against an id you already have — a job id, an export id —
//! and the toast is created on the first call and updated in place after:
//!
//! ```ignore
//! toast::progress(&mut commands, "export", "Rendering audio…", Some(0.4));
//! // …later, from anywhere, with no Entity held in between:
//! toast::progress(&mut commands, "export", "Writing file…", Some(0.9));
//! toast::complete(&mut commands, "export", "Exported", true);
//! ```
//!
//! `None` for the fraction means indeterminate — the bar shows, with nothing
//! to say about how far along it is.
//!
//! A toast tracking work never auto-dismisses; [`complete`] restarts its
//! countdown from that moment, so a message that lands after a long job gets
//! its full [`ToastDuration`] on screen rather than inheriting a timer that
//! started minutes ago. [`ToastState`] carries all of this.

use bevy::prelude::*;

use crate::theme::ThemePlugin;

mod components;
mod spawn;
mod systems;

pub use components::{
    ProgressToast, Toast, ToastCreated, ToastDismissible, ToastDuration, ToastMessage, ToastNodes,
    ToastPosition, ToastShowProgress, ToastState, ToastTitle, ToastVariant,
};
pub use spawn::{ToastSpec, complete, progress, spawn, spawn_error};

/// Toast-stack configuration. Mutate to change where toasts anchor,
/// how wide they render, or the max-on-screen ceiling.
#[derive(Resource, Clone, Debug)]
pub struct ToastConfig {
    pub position: ToastPosition,
    pub width: f32,
    pub max_toasts: usize,
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self {
            position: ToastPosition::default(),
            width: 356.0,
            max_toasts: 5,
        }
    }
}

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.init_resource::<ToastConfig>().add_systems(
            Update,
            (systems::reconcile_toast_ui, systems::tick_toasts).chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A toast app with no window, so `tick_toasts` returns early and these
    /// tests drive only the state model — which state a toast is in and what
    /// that implies, not where it renders.
    ///
    /// `Time` is inserted directly rather than via `TimePlugin`, matching
    /// [`crate::switch`]: the plugin overwrites the generic clock from
    /// `Time<Virtual>` each frame, which clobbers a manual `advance_by`.
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>().add_plugins(ToastPlugin);
        app
    }

    /// An app whose `tick_toasts` actually runs.
    ///
    /// The tick bails without a `PrimaryWindow`, so anything about dismissal
    /// timing needs one. `Window` is an ordinary component — spawning it
    /// headlessly is enough, no windowing backend involved.
    fn ticking_app() -> App {
        let mut app = app();
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app
    }

    /// Move the clock on by `secs` and run a frame.
    fn advance(app: &mut App, secs: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(secs));
        app.update();
    }

    fn alive(app: &App, toast: Entity) -> bool {
        app.world().get_entity(toast).is_ok()
    }

    fn state_of(app: &App, toast: Entity) -> Option<ToastState> {
        app.world().get::<ToastState>(toast).copied()
    }

    fn message_of(app: &App, toast: Entity) -> Option<String> {
        app.world().get::<ToastMessage>(toast).map(|m| m.0.clone())
    }

    /// The one live toast tracking `op_id`.
    fn tracked(app: &mut App, op_id: &str) -> Option<Entity> {
        app.world_mut()
            .query_filtered::<(Entity, &ProgressToast), With<Toast>>()
            .iter(app.world())
            .find(|(_, id)| id.0 == op_id)
            .map(|(e, _)| e)
    }

    fn toast_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<Toast>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn a_plain_toast_counts_down() {
        let mut app = app();
        let toast = {
            let mut commands = app.world_mut().commands();
            spawn(&mut commands, "Saved")
        };
        app.update();

        assert_eq!(state_of(&app, toast), Some(ToastState::Timed));
    }

    #[test]
    fn reporting_progress_creates_one_toast_and_then_reuses_it() {
        // The property the op-id exists for: a caller reports against an id it
        // already has and never holds an `Entity` between calls.
        let mut app = app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Starting…", Some(0.0));
        });
        app.update();
        assert_eq!(toast_count(&mut app), 1);
        let first = tracked(&mut app, "export").expect("created on first report");

        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Halfway", Some(0.5));
        });
        app.update();

        assert_eq!(toast_count(&mut app), 1, "a second report must not spawn");
        assert_eq!(tracked(&mut app, "export"), Some(first), "same entity");
        assert_eq!(message_of(&app, first).as_deref(), Some("Halfway"));
        assert_eq!(
            state_of(&app, first),
            Some(ToastState::Working {
                progress: Some(0.5)
            })
        );
    }

    #[test]
    fn two_operations_get_two_toasts() {
        let mut app = app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Exporting", Some(0.1));
            progress(&mut commands, "import", "Importing", Some(0.2));
        });
        app.update();

        assert_eq!(toast_count(&mut app), 2);
        assert_ne!(tracked(&mut app, "export"), tracked(&mut app, "import"));
    }

    #[test]
    fn work_in_flight_holds_the_toast_open() {
        // The distinction the old `ToastExternalProgress` conflated: a
        // fraction and "do not dismiss" are separate facts.
        assert!(
            ToastState::Working {
                progress: Some(0.5)
            }
            .holds_open()
        );
        assert!(ToastState::Working { progress: None }.holds_open());
        assert!(!ToastState::Timed.holds_open());
        assert!(!ToastState::Done.holds_open());
    }

    #[test]
    fn indeterminate_work_has_no_fraction_but_still_holds_open() {
        // Unrepresentable in the old model — holding a toast open required
        // inventing a progress number.
        let state = ToastState::Working { progress: None };
        assert_eq!(state.progress(), None);
        assert!(state.holds_open());
    }

    #[test]
    fn a_countdown_state_has_no_fraction_of_its_own() {
        // `Timed` and `Done` draw the countdown, which is the tick's business.
        assert_eq!(ToastState::Timed.progress(), None);
        assert_eq!(ToastState::Done.progress(), None);
    }

    #[test]
    fn completing_marks_it_done_and_releases_the_id() {
        let mut app = app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Exporting", Some(0.9));
        });
        app.update();
        let toast = tracked(&mut app, "export").expect("created");

        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            complete(&mut commands, "export", "Exported", true);
        });
        app.update();

        assert_eq!(state_of(&app, toast), Some(ToastState::Done));
        assert_eq!(message_of(&app, toast).as_deref(), Some("Exported"));
        assert!(
            !app.world().entity(toast).contains::<ProgressToast>(),
            "the id must be released so later work can reuse it"
        );
    }

    #[test]
    fn a_failed_completion_turns_destructive() {
        let mut app = app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Exporting", Some(0.5));
        });
        app.update();
        let toast = tracked(&mut app, "export").expect("created");

        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            complete(&mut commands, "export", "Export failed", false);
        });
        app.update();

        assert_eq!(
            app.world().get::<ToastVariant>(toast).copied(),
            Some(ToastVariant::Destructive)
        );
    }

    #[test]
    fn completing_an_operation_with_no_toast_still_says_something() {
        // Work that finished faster than its first progress report, or whose
        // toast was already dismissed. The outcome must not vanish.
        let mut app = app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            complete(&mut commands, "never-started", "Done anyway", true);
        });
        app.update();

        assert_eq!(toast_count(&mut app), 1);
    }

    #[test]
    fn a_released_id_can_be_reused_by_later_work() {
        let mut app = app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "First run", Some(0.5));
        });
        app.update();
        let first = tracked(&mut app, "export").expect("created");

        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            complete(&mut commands, "export", "First done", true);
        });
        app.update();

        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Second run", Some(0.1));
        });
        app.update();

        let second = tracked(&mut app, "export").expect("a fresh toast");
        assert_ne!(
            second, first,
            "a completed toast must not be adopted by the next operation"
        );
    }

    #[test]
    fn a_timed_toast_dismisses_when_its_duration_elapses() {
        let mut app = ticking_app();
        let toast = {
            let mut commands = app.world_mut().commands();
            ToastSpec::new("Saved")
                .duration(Duration::from_secs(2))
                .spawn(&mut commands)
        };
        app.update();
        assert!(alive(&app, toast), "still on screen at t=0");

        advance(&mut app, 3.0);
        assert!(!alive(&app, toast), "gone after its duration");
    }

    #[test]
    fn work_in_flight_outlives_its_duration() {
        // The reason `Working` suppresses the countdown at all: a job that
        // takes longer than four seconds must not have its toast vanish
        // mid-flight.
        let mut app = ticking_app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Exporting", Some(0.5));
        });
        app.update();
        let toast = tracked(&mut app, "export").expect("created");

        advance(&mut app, 60.0);

        assert!(
            alive(&app, toast),
            "a toast tracking work must not auto-dismiss"
        );
    }

    #[test]
    fn a_finished_toast_gets_its_full_time_on_screen() {
        // The bug `Done` exists to fix. Completion used to drop the
        // hold-open flag and leave the countdown running from `ToastCreated`
        // — first appearance, here a minute earlier — so the result message
        // expired on the frame it arrived and the user never saw it.
        let mut app = ticking_app();
        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            progress(&mut commands, "export", "Exporting", Some(0.1));
        });
        app.update();
        let toast = tracked(&mut app, "export").expect("created");

        // A long job.
        advance(&mut app, 60.0);
        assert!(alive(&app, toast), "held open while working");

        app.world_mut().commands().queue(|world: &mut World| {
            let mut commands = world.commands();
            complete(&mut commands, "export", "Exported", true);
        });
        app.update();

        // Well past the 4s default, but only just after completion.
        advance(&mut app, 1.0);
        assert!(
            alive(&app, toast),
            "the countdown must restart at completion, not resume a stale one"
        );

        advance(&mut app, 5.0);
        assert!(!alive(&app, toast), "and then expire normally");
    }

    #[test]
    fn the_builder_still_spells_a_progress_toast() {
        let mut app = app();
        let toast = {
            let mut commands = app.world_mut().commands();
            ToastSpec::new("Rendering")
                .progress(0.25)
                .duration(Duration::from_secs(9))
                .spawn(&mut commands)
        };
        app.update();

        assert_eq!(
            state_of(&app, toast),
            Some(ToastState::Working {
                progress: Some(0.25)
            })
        );
    }
}
