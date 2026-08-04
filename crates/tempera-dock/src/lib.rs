//! A recursive split-tree dock for a content-agnostic application shell.
//!
//! This crate owns the *layout* — panes, splits, dividers, resize, collapse,
//! and the persisted shape of all four. It knows nothing about what a pane
//! contains. An IDE's file tree, a graphics editor's layer list and a DAW's
//! mixer are the same thing from here: a rectangle with a name.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use tempera_dock::{Axis, DockLayout, DockTree, TemperaDockPlugin};
//!
//! let mut app = App::new();
//! app.add_plugins(TemperaDockPlugin)
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
//! back: tempera-dock never learns that `"browser"` is a file tree. So a pane
//! is found the way a command is found in [`tempera_input`] — by id:
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
//! # Several contents in one pane
//!
//! See [`page`]. A pane can hold N candidate contents with one showing, which
//! is what a tab bar, a mode switcher or a stacked view all reduce to.
//!
//! Note that this is **not** a node kind: there is no `DockTree::Tabs` variant
//! and there will not be one. [`DockTree`] answers how the window divides, and
//! switching page divides nothing — no divider moves and nothing resizes. So
//! pages are components on a pane's children, and this tree is untouched.
//!
//! Drag-to-rearrange is not implemented, and that one *is* a tree change. The
//! design keeps it cheap: resize is positional, so reordering a split's
//! children needs no divider bookkeeping, and a pane frame is a distinct entity
//! from its content, so a move is one reparent with the content subtree
//! following.
//!
//! [`tempera_input`]: https://docs.rs/tempera-input

#![forbid(unsafe_code)]

pub mod build;
pub mod mutate;
pub mod node;
pub mod page;
pub mod page_strip;
pub mod persist;
pub mod plugin;
pub mod registry;
pub mod resize;
pub mod section;
pub mod tree;
pub mod visibility;
pub mod window_chrome;

pub use build::DockBuildSet;
pub use mutate::{DockCommands, Side};
pub use node::{
    Axis, Divider, DockNode, DockRoot, Pane, PaneId, PaneMinSize, PaneSize, PaneVisibility, Split,
};
pub use page::{ActivePage, Page, PageIcon, PageId, PageLabel, PageOrder, page_is_active};
pub use page_strip::{PageChip, PageStrip, PageStripStyle};
pub use plugin::TemperaDockPlugin;
pub use registry::{PaneRegistry, pane_exists};
pub use resize::{DividerStyle, redistribute_flex};
pub use section::{
    Row, RowHeight, Section, SectionBody, SectionCell, SectionFrame, SectionId, SectionLabel,
    SectionOrder, SectionRow, SectionStack, SectionStyle, add_row,
};
pub use tree::{DockLayout, DockTree, TreeError};
