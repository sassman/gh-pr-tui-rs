//! Main View State

use crate::domain_models::Repository;

/// Main view state
#[derive(Debug, Clone, Default)]
pub struct MainViewState {
    pub selected_repository: usize, // Currently selected repository index
    pub repositories: Vec<Repository>, // List of tracked repositories
    pub repo_data: std::collections::HashMap<usize, RepositoryData>, // PR data per repository

    // Session restoration - pending selection to apply after repositories load
    /// Pending repository selection from session (org, name, branch, host)
    /// host is None for github.com repositories
    pub pending_session_repo: Option<(String, String, String, Option<String>)>,
    /// Pending PR number from session (not index)
    pub pending_session_pr_no: Option<usize>,
}

/// Data for a single repository (PRs, loading state, etc.)
#[derive(Debug, Clone, Default)]
pub struct RepositoryData {
    /// List of pull requests for this repository
    pub prs: Vec<crate::domain_models::Pr>,
    /// Current loading state
    pub loading_state: crate::domain_models::LoadingState,
    /// Currently selected PR index in the table (cursor position)
    pub selected_pr: usize,
    /// Set of selected PR numbers for bulk operations
    pub selected_pr_numbers: std::collections::HashSet<usize>,
    /// Timestamp of last successful load
    pub last_updated: Option<chrono::DateTime<chrono::Local>>,
    /// Current filter for displaying PRs
    pub current_filter: PrFilter,
}

/// PR filter for displaying only matching PRs
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PrFilter {
    /// Show all PRs (no filtering)
    #[default]
    All,
    /// Show only PRs that are ready to merge
    ReadyToMerge,
    /// Show only PRs that need rebase
    NeedsRebase,
    /// Show only PRs with failed builds
    BuildFailed,
    /// Show only PRs authored by the current user
    MyPRs,
    /// Custom text filter (matches title or author)
    Custom(String),
}

impl PrFilter {
    /// Get the display label for this filter
    pub fn label(&self) -> &str {
        match self {
            Self::All => "All",
            Self::ReadyToMerge => "Ready to Merge",
            Self::NeedsRebase => "Needs Rebase",
            Self::BuildFailed => "Build Failed",
            Self::MyPRs => "My PRs",
            Self::Custom(_) => "Custom",
        }
    }

    /// Cycle to the next filter in the preset sequence
    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::ReadyToMerge,
            Self::ReadyToMerge => Self::NeedsRebase,
            Self::NeedsRebase => Self::BuildFailed,
            Self::BuildFailed => Self::MyPRs,
            Self::MyPRs => Self::All,
            Self::Custom(_) => Self::All,
        }
    }
}
