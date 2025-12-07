//! State management for the diff viewer widget.

mod comment_editor;
mod navigation;
mod viewer_state;

pub use comment_editor::CommentEditor;
pub use navigation::{NavigationState, SelectionMode};
pub use viewer_state::DiffViewerState;
