//! # gh-diff-viewer
//!
//! A standalone, reusable diff viewer crate for GitHub PR code reviews with
//! syntax highlighting, context expansion, and line commenting capabilities.
//!
//! ## Design Principles
//!
//! This crate is designed to be **instrumented** — it receives data and emits
//! events without directly calling external APIs. This enables:
//!
//! - Testability without mocking HTTP clients
//! - Reusability in different contexts (GitHub, GitLab, local git)
//! - Clear separation of concerns
//!
//! ## Action-Based Architecture
//!
//! The viewer uses a tagged action pattern. Instead of handling key events directly,
//! the orchestrating application maps key events to [`DiffAction`] variants and
//! dispatches them to the viewer state. This allows integration with any key
//! handling system.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use gh_diff_viewer::{DiffViewer, DiffViewerState, DiffAction, PullRequestDiff};
//! use gh_diff_viewer::parser::parse_unified_diff;
//!
//! // Parse a unified diff
//! let diff = parse_unified_diff(diff_text, "base_sha", "head_sha")?;
//!
//! // Create state
//! let mut state = DiffViewerState::new(diff);
//!
//! // Render the widget
//! let widget = DiffViewer::new(&mut highlighter, &theme);
//! widget.render_with_state(area, buf, &mut state);
//!
//! // Handle actions (mapped from key events by the orchestrator)
//! let events = state.handle_action(DiffAction::CursorDown);
//! for event in events {
//!     // Process DiffEvent (e.g., submit comment, fetch context)
//! }
//! ```

pub mod action;
pub mod event;
pub mod highlight;
pub mod model;
pub mod parser;
pub mod state;
pub mod traits;
pub mod widget;

// Re-export commonly used types
pub use action::DiffAction;
pub use event::DiffEvent;
pub use highlight::DiffHighlighter;
pub use model::{
    CommentPosition, DiffLine, DiffSide, FileDiff, FileStatus, Hunk, LineKind, PendingComment,
    PullRequestDiff, ReviewEvent,
};
pub use parser::parse_unified_diff;
pub use state::DiffViewerState;
pub use traits::{
    CommentError, CommentHandler, ContextError, ContextProvider, DefaultTheme, ThemeProvider,
};
pub use widget::{DiffViewer, FooterHint};
