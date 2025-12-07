//! Diff Viewer State
//!
//! Wrapper state for the diff viewer panel in gh-pr-lander.
//! This delegates to gh_diff_viewer's DiffViewerState for the actual diff logic.

use gh_diff_viewer::{DiffHighlighter, DiffViewerState as InnerState, PullRequestDiff};

/// Loading state for the diff viewer
#[derive(Debug, Clone, Default)]
pub enum DiffViewerLoadingState {
    /// Not loaded / idle
    #[default]
    Idle,
    /// Currently loading diff
    Loading,
    /// Diff loaded successfully
    Loaded,
    /// Loading failed with error
    Error(String),
}

/// State for the diff viewer panel
#[derive(Debug)]
pub struct DiffViewerState {
    /// The inner diff viewer state from gh-diff-viewer crate
    pub inner: Option<InnerState>,
    /// Syntax highlighter (shared across loads)
    pub highlighter: DiffHighlighter,
    /// Loading state
    pub loading: DiffViewerLoadingState,
    /// PR context
    pub pr_number: Option<u64>,
    pub pr_title: Option<String>,
    /// Head SHA for API calls (comments)
    pub head_sha: Option<String>,
}

impl Default for DiffViewerState {
    fn default() -> Self {
        Self {
            inner: None,
            highlighter: DiffHighlighter::new(),
            loading: DiffViewerLoadingState::default(),
            pr_number: None,
            pr_title: None,
            head_sha: None,
        }
    }
}

impl Clone for DiffViewerState {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            highlighter: DiffHighlighter::new(), // Highlighter is not Clone, create new
            loading: self.loading.clone(),
            pr_number: self.pr_number,
            pr_title: self.pr_title.clone(),
            head_sha: self.head_sha.clone(),
        }
    }
}

impl DiffViewerState {
    /// Create a new empty diff viewer state
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a diff into the viewer
    pub fn load(
        &mut self,
        diff: PullRequestDiff,
        pr_number: u64,
        pr_title: String,
        head_sha: String,
    ) {
        self.inner = Some(InnerState::new(diff));
        self.loading = DiffViewerLoadingState::Loaded;
        self.pr_number = Some(pr_number);
        self.pr_title = Some(pr_title);
        self.head_sha = Some(head_sha);
    }

    /// Set loading state
    pub fn set_loading(&mut self) {
        self.loading = DiffViewerLoadingState::Loading;
    }

    /// Set error state
    pub fn set_error(&mut self, error: String) {
        self.loading = DiffViewerLoadingState::Error(error);
        self.inner = None;
    }

    /// Check if a diff is loaded
    pub fn is_loaded(&self) -> bool {
        matches!(self.loading, DiffViewerLoadingState::Loaded) && self.inner.is_some()
    }

    /// Check if currently loading
    pub fn is_loading(&self) -> bool {
        matches!(self.loading, DiffViewerLoadingState::Loading)
    }

    /// Get error message if in error state
    pub fn error_message(&self) -> Option<&str> {
        match &self.loading {
            DiffViewerLoadingState::Error(msg) => Some(msg),
            _ => None,
        }
    }

    /// Reset to idle state
    pub fn reset(&mut self) {
        self.inner = None;
        self.loading = DiffViewerLoadingState::Idle;
        self.pr_number = None;
        self.pr_title = None;
        self.head_sha = None;
    }
}
