//! Pull Request screen actions
//!
//! Actions specific to the main PR view screen.

use crate::domain_models::{MergeableStatus, Pr, Repository};
use crate::state::PrFilter;

/// Actions for the Pull Request screen
#[derive(Debug, Clone)]
pub enum PullRequestAction {
    // Navigation (translated from NavigationAction)
    /// Navigate to next PR in the table
    NavigateNext,
    /// Navigate to previous PR in the table
    NavigatePrevious,
    /// Navigate to top of PR list
    NavigateToTop,
    /// Navigate to bottom of PR list
    NavigateToBottom,

    // Repository switching
    /// Switch to next repository tab
    RepositoryNext,
    /// Switch to previous repository tab
    RepositoryPrevious,

    // PR Loading
    /// Start loading PRs for a repository
    LoadStart { repo: Repository },
    /// PRs loaded successfully for a repository
    Loaded { repo: Repository, prs: Vec<Pr> },
    /// Failed to load PRs for a repository
    LoadError { repo: Repository, error: String },

    // Selection
    /// Toggle selection of the current PR (at cursor)
    ToggleSelection,
    /// Select all PRs in the current repository
    SelectAll,
    /// Deselect all PRs in the current repository
    DeselectAll,

    // Operations
    /// Open current PR in browser
    OpenInBrowser,
    /// Open current PR diff in configured IDE
    OpenInIDE,
    /// Open CI build logs in browser
    OpenBuildLogs,
    /// Refresh PRs for the current repository
    Refresh,
    // Merge operations
    /// Request to merge selected PRs (or cursor PR if none selected)
    MergeRequest,
    /// Merge started for a PR
    MergeStart { repo: Repository, pr_number: usize },

    // Rebase operations
    /// Request to rebase/update selected PRs
    RebaseRequest,
    /// Rebase started for a PR
    RebaseStart { repo: Repository, pr_number: usize },

    // Approve operations
    /// Request to approve selected PRs (shows confirmation popup)
    ApproveRequest,
    /// Request to comment on selected PRs (shows confirmation popup)
    CommentRequest,
    /// Request to request changes on selected PRs (shows confirmation popup)
    RequestChangesRequest,
    /// Approve PRs with a custom message (from confirmation popup)
    ApproveWithMessage {
        pr_numbers: Vec<u64>,
        message: String,
    },
    /// Approve started for a PR
    ApproveStart { repo: Repository, pr_number: usize },

    // Comment operations
    /// Post a comment on PRs (from confirmation popup)
    CommentOnPr {
        pr_numbers: Vec<u64>,
        message: String,
    },
    /// Comment started for a PR
    CommentStart { repo: Repository, pr_number: usize },

    // Request changes operations
    /// Request changes on PRs (from confirmation popup)
    RequestChanges {
        pr_numbers: Vec<u64>,
        message: String,
    },
    /// Request changes started for a PR
    RequestChangesStart { repo: Repository, pr_number: usize },

    // Close operations
    /// Request to close selected PRs
    CloseRequest,
    /// Close PRs with a custom message (from confirmation popup)
    ClosePrWithMessage {
        pr_numbers: Vec<u64>,
        message: String,
    },
    /// Close started for a PR
    CloseStart { repo: Repository, pr_number: usize },

    // CI/Build Status actions
    /// Trigger a CI status check for a specific PR
    CheckBuildStatus {
        repo: Repository,
        pr_number: u64,
        head_sha: String,
    },
    /// Update the build status of a specific PR after CI check completes
    BuildStatusUpdated {
        repo: Repository,
        pr_number: u64,
        status: MergeableStatus,
    },
    /// Request to rerun failed jobs for the current PR
    RerunFailedJobs,
    /// Rerun started for a workflow run
    RerunStart {
        repo: Repository,
        pr_number: u64,
        run_id: u64,
    },

    // Filters
    /// Cycle through filter presets
    CycleFilter,
    /// Set a specific filter
    SetFilter(PrFilter),
    /// Clear the current filter (show all PRs)
    ClearFilter,
}
