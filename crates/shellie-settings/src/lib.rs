//! A tabbed settings dialog — the chrome, and nothing that goes in it.
//!
//! A sidebar of tabs, a scrollable body, and one tab showing at a time.
//! What each tab *contains* is the host's: this crate spawns an empty body
//! per declared tab and never looks inside one.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use shellie_settings::{ShellieSettingsPlugin, SettingsTab, TabId, TabLabel, TabOrder};
//!
//! let mut app = App::new();
//! app.add_plugins(ShellieSettingsPlugin);
//! app.world_mut().spawn((
//!     SettingsTab,
//!     TabId::from("general"),
//!     TabLabel::from("General"),
//!     TabOrder(10),
//! ));
//! ```
//!
//! # Why tabs are declared, not enumerated
//!
//! The dialog this was extracted from had a closed six-variant enum, and
//! adding a tab meant editing **four places that had to stay in lockstep**:
//! the enum, a hardcoded label table, a `SystemParam` with six named
//! queries, and two hand-written six-tuples. A single missed edit disabled
//! tab switching entirely, silently, because the six-way destructure
//! bailed.
//!
//! It also could not work here at all. A crate that names `Audio` and
//! `Extensions` is a crate that knows it is running a DAW. So a tab is an
//! entity a host spawns, ordered by [`TabOrder`] — which means an
//! *extension* can contribute one, something the enum made impossible.
//!
//! # Content finds its body; the dialog never pushes
//!
//! A panel queries for its body by string id and parents itself in
//! ([`TabBodies`]). That inversion is what lets a tab's content live in a
//! crate this one has never heard of, and it is why an unrecognised id is
//! *ignorable* rather than wrong — the same doctrine `shellie-input`
//! applies to a saved keybind naming a command that no longer exists.
//!
//! # What it refuses
//!
//! | not here | because |
//! | --- | --- |
//! | preference values | The crate never sees them. A control's write-back names a host type, so the binding is the host's — as `tree_row` leaves clicks to its caller. |
//! | persistence | Follows from the above: there is nothing here to persist but the active tab, and that is one string on the dialog entity. |
//! | the form rows | `tempera::setting_row` and `tempera::list_row`. This crate draws a sidebar; it does not draw controls. |
//! | opening the dialog | A host decision — a menu item, a keybind, a command. This crate mirrors [`SettingsOpen`] onto `Visibility` and never sets it. |
//! | the dialog chrome | `tempera::dialog` owns the card, the backdrop, Escape, and the close button. |
//!
//! # Closing is reported, not swallowed
//!
//! tempera fires `DialogDismissed` on Escape, a backdrop click or the close
//! button, and deliberately does not hide itself — its docs call this
//! "source-of-truth-agnostic". This crate preserves that: a host reads the
//! message and clears [`SettingsOpen`]. Swallowing it would mean a host
//! whose own state said "open" while the dialog had already closed itself.

#![forbid(unsafe_code)]

pub mod build;
pub mod layout;
pub mod node;
pub mod plugin;
pub mod registry;
mod systems;
pub mod tab;

pub use build::{SettingsBuildSet, SettingsCloseIcon, SettingsStyle, SettingsTitle, ordered_tabs};
pub use layout::SettingsLayout;
pub use node::{
    SettingsBody, SettingsContentRow, SettingsDialog, SettingsOpen, SettingsSidebar, SidebarEntry,
};
pub use plugin::ShellieSettingsPlugin;
pub use registry::{TabBodies, tab_exists};
pub use tab::{ActiveTab, SettingsTab, TabBody, TabIcon, TabId, TabLabel, TabOrder};
