//! Build Log Actions
//!
//! Tagged actions for the build log panel.

use crate::state::{BuildLogJobMetadata, BuildLogPrContext};

/// Tagged actions for the build log panel
#[derive(Debug, Clone)]
pub enum BuildLogAction {
    // === Loading ===
    /// Open build logs for current PR (triggers async fetch)
    Open,
    /// Loading started
    LoadStart,
    /// Logs loaded successfully
    Loaded {
        workflows: Vec<gh_actions_log_parser::WorkflowNode>,
        job_metadata: Vec<BuildLogJobMetadata>,
        pr_context: BuildLogPrContext,
    },
    /// Loading failed
    LoadError(String),

    // === Navigation ===
    /// Navigate to next visible node (down)
    NavigateDown,
    /// Navigate to previous visible node (up)
    NavigateUp,
    /// Navigate to first node
    NavigateToTop,
    /// Navigate to last node
    NavigateToBottom,

    // === Tree Operations ===
    /// Expand/collapse node at cursor
    Toggle,
    /// Expand all nodes
    ExpandAll,
    /// Collapse all nodes
    CollapseAll,

    // === Error Navigation ===
    /// Jump to next error
    NextError,
    /// Jump to previous error
    PrevError,

    // === Scrolling ===
    /// Scroll left (for long log lines)
    ScrollLeft,
    /// Scroll right (for long log lines)
    ScrollRight,
    /// Page down
    PageDown,
    /// Page up
    PageUp,

    // === View Options ===
    /// Toggle timestamp display
    ToggleTimestamps,

    // === Viewport ===
    /// Update viewport height (called during render)
    SetViewportHeight(usize),
}
