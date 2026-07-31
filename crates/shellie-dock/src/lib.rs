//! A recursive split-tree dock for a content-agnostic application shell.
//!
//! This crate owns the *layout* — panes, splits, dividers, resize, collapse,
//! and the persisted shape of all four. It knows nothing about what a pane
//! contains. An IDE's file tree, a graphics editor's layer list and a DAW's
//! mixer are the same thing from here: a rectangle with a name.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use shellie_dock::{Axis, DockLayout, DockTree, ShellieDockPlugin};
//!
//! let mut app = App::new();
//! app.add_plugins(ShellieDockPlugin)
//!     .insert_resource(DockLayout::new(DockTree::split(
//!         Axis::Column,
//!         [
//!             DockTree::pane("title_bar").fixed(40.0),
//!             DockTree::split(
//!                 Axis::Row,
//!                 [
//!                     DockTree::pane("browser").flex(1.0),
//!                     DockTree::pane("center").flex(6.0),
//!                     DockTree::pane("inspector").flex(1.0).min_size(120.0),
//!                 ],
//!             )
//!             .flex(1.0),
//!         ],
//!     )));
//! ```
//!
//! # Why a tree
//!
//! A fixed set of named slots — title bar, left panel, center, right panel —
//! is the layout every shell starts with and the one every shell outgrows. It
//! cannot express a split editor, a bottom console, or a second inspector,
//! and each new slot costs a marker type, a branch in the resize code, and a
//! field in the persisted state. A tree costs those once.
//!
//! The tree is **declared as data** ([`DockLayout`]), not spawned by a
//! function. That is what lets "the host declares a layout", "a saved layout
//! is restored" and "the user drags a pane somewhere else" be one code path
//! rather than three that must agree.
//!
//! # Why panes are identified by string
//!
//! Content lives in crates this one cannot name, and the arrow does not point
//! back: shellie-dock never learns that `"browser"` is a file tree. So a pane
//! is found the way a command is found in [`shellie_input`] — by id:
//!
//! ```ignore
//! let Some(pane) = panes.get("browser") else { return };
//! commands.spawn((MyPanel, /* … */)).insert(ChildOf(pane));
//! ```
//!
//! Content **finds** its pane; the dock never pushes content into one. An id
//! that no longer exists is therefore *ignorable* rather than wrong — the same
//! doctrine `SavedKeybinds` applies to a command id that has been removed.
//!
//! # Why there are no floating panes
//!
//! Overlays — a floating tool palette, a pill toolbar, a HUD — are real, and
//! they are **not** panes. They need z-order, viewport clamping and their own
//! drag behaviour, none of which is a split. Modelling them here would mean
//! two layout modes in one tree forever, and every future feature paying for
//! both.
//!
//! Instead a pane frame is `position: relative` with `overflow: clip`, so an
//! application can spawn an absolutely-positioned child of any pane and get an
//! overlay anchored to it. The dock provides the rectangle; the app decides
//! what floats over it.
//!
//! This is why resize is **positional** — a divider finds its neighbours by
//! index among its parent's children, never by marker. A layout where one pane
//! resizes against a sibling and another against the window is a layout where
//! rearranging panes means rewriting the resize code.
//!
//! # Tabs
//!
//! Not implemented. The design keeps them cheap: [`DockTree`] is recursive on
//! disk, so a `Tabs` variant is additive rather than a break; a pane *frame* is
//! a distinct entity from the content parented into it, so moving a pane is one
//! reparent and the content subtree follows; and resize is positional, so
//! reordering children needs no changes at all.
//!
//! [`shellie_input`]: https://docs.rs/shellie-input

#![forbid(unsafe_code)]

pub mod build;
pub mod center_mode;
pub mod mutate;
pub mod node;
pub mod persist;
pub mod plugin;
pub mod registry;
pub mod resize;
pub mod tree;
pub mod visibility;

pub use build::DockBuildSet;
pub use center_mode::{
    ActiveCenterMode, CenterContent, CenterMode, CenterModeIcon, CenterModeId, CenterModeLabel,
    CenterModeOrder, CenterModeRegistration, is_active,
};
pub use mutate::{DockCommands, Side};
pub use node::{
    Axis, Divider, DockNode, DockRoot, Pane, PaneId, PaneMinSize, PaneSize, PaneVisibility, Split,
};
pub use plugin::ShellieDockPlugin;
pub use registry::{PaneRegistry, pane_exists};
pub use resize::{DividerStyle, redistribute_flex};
pub use tree::{DockLayout, DockTree, TreeError};
