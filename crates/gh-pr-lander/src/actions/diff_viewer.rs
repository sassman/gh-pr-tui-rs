//! Diff Viewer Actions
//!
//! Tagged actions for the diff viewer panel.

use gh_diff_viewer::{DiffEvent, PullRequestDiff};

/// A review comment loaded from GitHub
#[derive(Debug, Clone)]
pub struct LoadedComment {
    /// GitHub comment ID
    pub github_id: u64,
    /// File path
    pub path: String,
    /// Line number (may be None for outdated comments)
    pub line: Option<u32>,
    /// Side: "LEFT" or "RIGHT"
    pub side: Option<String>,
    /// Comment body
    pub body: String,
}

/// Tagged actions for the diff viewer panel
#[derive(Debug, Clone)]
pub enum DiffViewerAction {
    // === Loading ===
    /// Open diff viewer for current PR (triggers async fetch)
    Open,
    /// Loading started
    LoadStart,
    /// Diff loaded successfully
    Loaded {
        diff: PullRequestDiff,
        pr_number: u64,
        pr_title: String,
        head_sha: String,
        /// Review comments loaded from GitHub
        comments: Vec<LoadedComment>,
    },
    /// Loading failed
    LoadError(String),

    // === Navigation (delegated from generic Navigate actions) ===
    /// Navigate to next item (file or line)
    NavigateDown,
    /// Navigate to previous item (file or line)
    NavigateUp,
    /// Navigate left (to file tree or previous pane)
    NavigateLeft,
    /// Navigate right (to diff content or next pane)
    NavigateRight,
    /// Navigate to top
    NavigateToTop,
    /// Navigate to bottom
    NavigateToBottom,
    /// Jump to next hunk header
    NextHunk,
    /// Jump to previous hunk header
    PrevHunk,

    // === Scrolling ===
    /// Page down
    PageDown,
    /// Page up
    PageUp,

    // === Tree Operations ===
    /// Expand/collapse file in tree
    Toggle,
    /// Expand all files
    ExpandAll,
    /// Collapse all files
    CollapseAll,

    // === Focus Management ===
    /// Switch focus between file tree and diff content
    SwitchPane,
    /// Escape key: if editing comment, cancel; if in diff pane, focus file tree
    EscapeOrFocusTree,

    // === Visual Mode ===
    /// Enter visual mode for line selection
    EnterVisualMode,
    /// Exit visual mode
    ExitVisualMode,

    // === Generic Input (mode-aware, reducer decides based on inner state) ===
    /// Generic key press - reducer routes based on mode (navigation vs comment editing)
    KeyPress(char),
    /// Backspace - deletes char in comment mode, no-op otherwise
    Backspace,
    /// Confirm - commits comment, selects file, or submits review based on mode
    Confirm,

    // === Comments (explicit actions when needed) ===
    /// Start adding a comment on current line
    AddComment,
    /// Cancel comment editing
    CancelComment,
    /// Commit the current comment (also triggered by Confirm in comment mode)
    CommitComment,
    /// Insert character into comment editor (also triggered by KeyPress in comment mode)
    CommentChar(char),
    /// Delete character from comment editor (also triggered by Backspace in comment mode)
    CommentBackspace,

    // === Review ===
    /// Show review popup
    ShowReviewPopup,
    /// Hide review popup
    HideReviewPopup,
    /// Navigate review popup options
    ReviewOptionNext,
    /// Navigate review popup options
    ReviewOptionPrev,
    /// Submit review with selected option (updates inner state, closes popup)
    SubmitReview,
    /// Request to submit review via API (handled by GitHub middleware)
    SubmitReviewRequest {
        pr_number: u64,
        event: gh_diff_viewer::ReviewEvent,
    },
    /// Request to submit a single line comment via API (handled by GitHub middleware)
    SubmitCommentRequest {
        pr_number: u64,
        head_sha: String,
        path: String,
        line: u32,
        side: String,
        body: String,
    },
    /// Comment was successfully posted to GitHub (updates local state with github_id)
    CommentPosted {
        path: String,
        line: u32,
        side: String,
        github_id: u64,
    },
    /// Request to delete a comment via API (handled by GitHub middleware)
    DeleteCommentRequest {
        pr_number: u64,
        github_id: u64,
        path: String,
        line: u32,
        side: String,
    },
    /// Comment was successfully deleted from GitHub
    CommentDeleted {
        path: String,
        line: u32,
        side: String,
    },

    // === Events from DiffViewerState ===
    /// Forward an event from the diff viewer state
    Event(DiffEvent),

    // === Viewport ===
    /// Update viewport dimensions
    SetViewport { width: u16, height: u16 },
}
