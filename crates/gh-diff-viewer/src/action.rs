//! Diff Viewer Actions
//!
//! Tagged actions that the diff viewer can process. These are exposed by the crate
//! so that the orchestrating application can transform key events into actions
//! and dispatch them to the viewer state.

/// Actions that can be performed on the diff viewer.
///
/// These are the semantic actions the viewer understands. The orchestrating
/// application is responsible for mapping key events to these actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffAction {
    // === Navigation ===
    /// Move cursor down one line
    CursorDown,
    /// Move cursor up one line
    CursorUp,
    /// Move to first line
    CursorFirst,
    /// Move to last line
    CursorLast,
    /// Move to next file
    NextFile,
    /// Move to previous file
    PrevFile,
    /// Select a specific file by index
    SelectFile(usize),
    /// Jump to next hunk header
    NextHunk,
    /// Jump to previous hunk header
    PrevHunk,

    // === Scrolling ===
    /// Scroll down half page
    ScrollHalfDown,
    /// Scroll up half page
    ScrollHalfUp,
    /// Scroll down full page
    ScrollPageDown,
    /// Scroll up full page
    ScrollPageUp,

    // === Focus Management ===
    /// Toggle focus between file tree and diff content
    ToggleFocus,
    /// Focus file tree pane
    FocusFileTree,
    /// Focus diff content pane
    FocusDiffContent,
    /// Toggle file tree visibility
    ToggleFileTree,

    // === File Tree Operations ===
    /// Toggle expand/collapse of current tree node
    ToggleTreeNode,
    /// Expand current tree node
    ExpandTreeNode,
    /// Collapse current tree node
    CollapseTreeNode,

    // === Visual Mode ===
    /// Enter visual selection mode
    EnterVisualMode,
    /// Exit visual selection mode
    ExitVisualMode,

    // === Comments ===
    /// Start adding a comment on current line/selection
    StartComment,
    /// Insert a character into the comment editor
    CommentInsertChar(char),
    /// Delete character before cursor in comment editor
    CommentBackspace,
    /// Delete character at cursor in comment editor
    CommentDelete,
    /// Move cursor left in comment editor
    CommentCursorLeft,
    /// Move cursor right in comment editor
    CommentCursorRight,
    /// Move cursor to start of line in comment editor
    CommentCursorHome,
    /// Move cursor to end of line in comment editor
    CommentCursorEnd,
    /// Insert newline in comment editor
    CommentNewline,
    /// Commit/save the current comment
    CommitComment,
    /// Cancel comment editing
    CancelComment,

    // === Review ===
    /// Show review submission popup
    ShowReviewPopup,
    /// Hide review submission popup
    HideReviewPopup,
    /// Select next review option
    ReviewOptionNext,
    /// Select previous review option
    ReviewOptionPrev,
    /// Submit the review with selected option
    SubmitReview,

    // === Context Expansion ===
    /// Expand context above current hunk
    ExpandContextAbove,
    /// Expand context below current hunk
    ExpandContextBelow,

    // === General ===
    /// Close the diff viewer
    Close,

    // === Viewport ===
    /// Set the viewport dimensions (for scroll calculations)
    SetViewport { width: u16, height: u16 },
}

impl DiffAction {
    /// Check if this action should be handled when in comment editing mode
    pub fn is_comment_action(&self) -> bool {
        matches!(
            self,
            DiffAction::CommentInsertChar(_)
                | DiffAction::CommentBackspace
                | DiffAction::CommentDelete
                | DiffAction::CommentCursorLeft
                | DiffAction::CommentCursorRight
                | DiffAction::CommentCursorHome
                | DiffAction::CommentCursorEnd
                | DiffAction::CommentNewline
                | DiffAction::CommitComment
                | DiffAction::CancelComment
        )
    }

    /// Check if this action is a navigation action
    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            DiffAction::CursorDown
                | DiffAction::CursorUp
                | DiffAction::CursorFirst
                | DiffAction::CursorLast
                | DiffAction::NextFile
                | DiffAction::PrevFile
                | DiffAction::SelectFile(_)
                | DiffAction::NextHunk
                | DiffAction::PrevHunk
        )
    }

    /// Check if this action is a scroll action
    pub fn is_scroll(&self) -> bool {
        matches!(
            self,
            DiffAction::ScrollHalfDown
                | DiffAction::ScrollHalfUp
                | DiffAction::ScrollPageDown
                | DiffAction::ScrollPageUp
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_comment_action() {
        assert!(DiffAction::CommentInsertChar('a').is_comment_action());
        assert!(DiffAction::CommentBackspace.is_comment_action());
        assert!(DiffAction::CommitComment.is_comment_action());
        assert!(!DiffAction::CursorDown.is_comment_action());
    }

    #[test]
    fn test_is_navigation() {
        assert!(DiffAction::CursorDown.is_navigation());
        assert!(DiffAction::NextFile.is_navigation());
        assert!(!DiffAction::ScrollPageDown.is_navigation());
    }

    #[test]
    fn test_is_scroll() {
        assert!(DiffAction::ScrollPageDown.is_scroll());
        assert!(DiffAction::ScrollHalfUp.is_scroll());
        assert!(!DiffAction::CursorDown.is_scroll());
    }
}
