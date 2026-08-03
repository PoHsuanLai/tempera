//! Leaving room for the OS's own window buttons.
//!
//! # The problem
//!
//! macOS draws the close / minimise / maximise buttons — the "traffic
//! lights" — *on top of* the window, in its top-left corner. An app that
//! draws a custom title bar there gets its own content overlapped, so it has
//! to leave a hole.
//!
//! The hole is not constant. **In fullscreen the traffic lights are hidden**,
//! and a bar that keeps reserving space for them looks wrong — content
//! indented for no visible reason. So the inset has to follow the window
//! mode, not just the platform.
//!
//! Windows and Linux draw their decorations outside the client area, so
//! there is nothing to avoid and the inset is ordinary edge padding.
//!
//! # Why this lives in the dock
//!
//! The dock owns the title-bar slot — it decides where the bar goes and how
//! tall it is. What has to be kept clear at the leading edge of that slot is
//! the same kind of fact, and a consumer that positions its first control
//! should be able to ask rather than rediscover.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};

/// How much space to leave clear at the leading edge of the title bar.
///
/// Read this rather than hardcoding an inset; it is kept current by
/// [`sync_window_inset`].
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct WindowChromeInset {
    /// Logical pixels to keep clear on the left.
    pub leading: f32,
}

impl WindowChromeInset {
    /// Width of the macOS traffic-light cluster plus its margin.
    ///
    /// Measured from the platform rather than derived from the spacing
    /// scale: it has to clear buttons whose size Apple chose, so a base
    /// change must not move it. This is the one number here that answers to
    /// something outside the theme.
    pub const MACOS_TRAFFIC_LIGHTS: f32 = 72.0;

    /// Ordinary edge padding, used where no OS buttons overlap the client
    /// area — every non-macOS platform, and macOS in fullscreen.
    pub const BARE: f32 = 6.0;
}

impl Default for WindowChromeInset {
    fn default() -> Self {
        Self {
            leading: if cfg!(target_os = "macos") {
                Self::MACOS_TRAFFIC_LIGHTS
            } else {
                Self::BARE
            },
        }
    }
}

/// Collapse the inset when the traffic lights are not on screen.
///
/// Reads [`Window::mode`] rather than comparing the window's size against the
/// monitor's. The size comparison is the obvious approach and it is a
/// heuristic: a borderless window sized exactly to the display reads as
/// fullscreen and loses its inset while the buttons are still drawn. The
/// window mode is the fact itself.
///
/// A no-op off macOS, where the resource never changes from its default.
pub fn sync_window_inset(
    windows: Query<Ref<Window>, With<PrimaryWindow>>,
    mut inset: ResMut<WindowChromeInset>,
) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    if !window.is_changed() {
        return;
    }
    let want = if matches!(window.mode, WindowMode::Windowed) {
        WindowChromeInset::MACOS_TRAFFIC_LIGHTS
    } else {
        WindowChromeInset::BARE
    };
    if inset.leading != want {
        inset.leading = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_reserves_room_only_where_the_os_overlaps() {
        // The whole point: on macOS the buttons are drawn over the client
        // area and must be avoided; elsewhere they are outside it.
        let inset = WindowChromeInset::default();
        if cfg!(target_os = "macos") {
            assert_eq!(inset.leading, WindowChromeInset::MACOS_TRAFFIC_LIGHTS);
        } else {
            assert_eq!(inset.leading, WindowChromeInset::BARE);
        }
    }

    #[test]
    fn the_inset_is_never_negative_or_absurd() {
        // A weak assertion, and deliberately so — see the module test note.
        // It at least catches a constant edited into nonsense.
        for v in [
            WindowChromeInset::MACOS_TRAFFIC_LIGHTS,
            WindowChromeInset::BARE,
        ] {
            assert!(v > 0.0 && v < 200.0, "{v} is not a plausible inset");
        }
    }

    #[test]
    fn syncing_off_macos_touches_nothing_at_all() {
        // Not "the value is still BARE" — off macOS the default is already
        // BARE, so that assertion passes with the platform guard removed and
        // proves nothing. What the guard actually buys is that the resource
        // is never *written*: a `ResMut` deref marks it changed, and anything
        // gated on `Res<WindowChromeInset>::is_changed()` would then re-run
        // on every window event on platforms that have no traffic lights.
        //
        // So this seeds a value the sync would overwrite if it ran, and
        // checks both that it survives and that nothing was dirtied.
        #[derive(Resource, Default)]
        struct Dirtied(usize);

        fn count(mut d: ResMut<Dirtied>, inset: Res<WindowChromeInset>) {
            if inset.is_changed() {
                d.0 += 1;
            }
        }

        let mut app = App::new();
        app.insert_resource(WindowChromeInset { leading: 99.0 })
            .init_resource::<Dirtied>()
            .add_systems(Update, (sync_window_inset, count).chain());
        app.world_mut().spawn((
            Window {
                mode: WindowMode::Fullscreen(
                    bevy::window::MonitorSelection::Primary,
                    bevy::window::VideoModeSelection::Current,
                ),
                ..default()
            },
            PrimaryWindow,
        ));
        app.update();
        app.world_mut().resource_mut::<Dirtied>().0 = 0;
        app.update();

        if !cfg!(target_os = "macos") {
            assert_eq!(
                app.world().resource::<WindowChromeInset>().leading,
                99.0,
                "a non-macOS build overwrote an inset it does not own"
            );
            assert_eq!(
                app.world().resource::<Dirtied>().0,
                0,
                "a non-macOS build dirtied the inset resource"
            );
        }
    }
}
