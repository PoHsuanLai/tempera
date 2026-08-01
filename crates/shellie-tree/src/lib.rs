//! A tree view's hierarchy, expansion and filtering — and nothing else.
//!
//! A file browser, a layer list, a project outline and a device tree all want
//! the same four things: **hierarchy** (who is whose child), **expansion**
//! (what is open), **filtering** (what survives a query) and **indent** (how
//! deep). This crate is those four, and the row they produce is drawn by
//! somebody else.
//!
//! ```ignore
//! for row in visible_rows(&nodes, &state, query) {
//!     let spec = TreeRowSpec::new(name_of(row.item)).depth(row.depth);
//!     spawn_tree_row(&mut commands, &style, spec);
//! }
//! ```
//!
//! # It computes; it does not spawn
//!
//! [`visible_rows`] is a pure function returning a flat `Vec<VisibleRow>` in
//! render order. It takes no `World`, holds no `Commands`, and creates no
//! entity. That is how real tree views work — VS Code, Chrome DevTools,
//! IntelliJ and every file manager keep a flat visible array and splice
//! collapsed subtrees out of it, rather than keeping a container element per
//! group and hiding it. A collapsed folder should cost nothing, not one UI node
//! per file inside it.
//!
//! The trade is stated plainly in [`visible`]: toggling a group re-emits the
//! list, so rows respawn where a container model would have flipped a flag.
//!
//! # What it refuses
//!
//! The crate is defined as much by what it will not model:
//!
//! | not here | because |
//! | --- | --- |
//! | icons, tints | Row decoration. The renderer takes an image; the tree never has an opinion about which. |
//! | label suffixes, badges | Host metadata. The tree needs *a* name, to sort and to match. |
//! | sibling order policy | Groups before leaves, then alphabetical. A host wanting otherwise sorts its own rows. |
//! | drag, context menus, activation | Rows are the host's entities; it observes them directly. None of that is hierarchy. |
//! | the search field | The host owns its text input and passes a string. |
//! | keyboard navigation | Genuinely missing, not refused — see below. |
//!
//! The measure: of the seven arguments the browser's row spawner took, this
//! crate supplies **two** — a name and a depth. The rest was host data passing
//! through a function that had no use for it.
//!
//! # Items are discovered, not declared
//!
//! [`shellie-dock`] takes a hand-authored layout value, because a dock
//! arrangement *is* authored — a human writes ten panes and it round-trips to
//! disk. A browser tree is the opposite: a filesystem walk mints a group per
//! directory prefix, and a plugin scan adds more, asynchronously, after the
//! window is up. So the hierarchy is read off components the host's scanner
//! puts on its own entities ([`item`]), and only *expansion* persists.
//!
//! The honest cost of that choice: the hierarchy cannot be validated up front
//! the way a declared layout can. A duplicate [`GroupId`] or a [`ParentGroup`]
//! cycle is detectable only while walking, so [`visible_rows`] warns and drops
//! the offending row rather than pretending it caught the problem earlier.
//!
//! # Sections are just root groups
//!
//! A "section" — a top-level collapsible band with a header — is a group with
//! no parent. It needs no separate type, no second open/closed mechanism and no
//! per-section match arms. The browser this came from had all three, with the
//! opposite default from its group state; see [`state`] for why one flat set of
//! open ids could not express that and two sets can.
//!
//! [`shellie-dock`]: https://docs.rs/shellie-dock

pub mod filter;
pub mod item;
pub mod persist;
pub mod query;
pub mod state;
pub mod visible;

pub use filter::{FilterNode, matches, surviving};
pub use item::{DefaultOpen, GroupId, ParentGroup, TreeId, TreeItem, TreeName};
pub use query::TreeQuery;
pub use state::{FORMAT_VERSION, TreeState};
pub use visible::{TreeNode, VisibleRow, visible_rows};
