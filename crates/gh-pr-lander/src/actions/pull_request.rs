//! Pull Request screen actions
//!
//! Actions specific to the main PR view screen.

use crate::domain_models::Pr;
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
    /// Start loading PRs for a repository (repo_index)
    LoadStart(usize),
    /// PRs loaded successfully for a repository (repo_index, prs)
    Loaded(usize, Vec<Pr>),
    /// Failed to load PRs for a repository (repo_index, error_message)
    LoadError(usize, String),

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

    /// Open the current repository in the browser
    OpenRepositoryInBrowser,

    // Merge operations
    /// Request to merge selected PRs (or cursor PR if none selected)
    MergeRequest,
    /// Merge started for a PR (repo_idx, pr_number)
    MergeStart(usize, usize),
    /// Merge succeeded (repo_idx, pr_number)
    MergeSuccess(usize, usize),
    /// Merge failed (repo_idx, pr_number, error)
    MergeError(usize, usize, String),

    // Rebase operations
    /// Request to rebase/update selected PRs
    RebaseRequest,
    /// Rebase started for a PR (repo_idx, pr_number)
    RebaseStart(usize, usize),
    /// Rebase succeeded (repo_idx, pr_number)
    RebaseSuccess(usize, usize),
    /// Rebase failed (repo_idx, pr_number, error)
    RebaseError(usize, usize, String),

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
    /// Approve started for a PR (repo_idx, pr_number)
    ApproveStart(usize, usize),
    /// Approve succeeded (repo_idx, pr_number)
    ApproveSuccess(usize, usize),
    /// Approve failed (repo_idx, pr_number, error)
    ApproveError(usize, usize, String),

    // Comment operations
    /// Post a comment on PRs (from confirmation popup)
    CommentOnPr {
        pr_numbers: Vec<u64>,
        message: String,
    },
    /// Comment started for a PR (repo_idx, pr_number)
    CommentStart(usize, usize),
    /// Comment succeeded (repo_idx, pr_number)
    CommentSuccess(usize, usize),
    /// Comment failed (repo_idx, pr_number, error)
    CommentError(usize, usize, String),

    // Request changes operations
    /// Request changes on PRs (from confirmation popup)
    RequestChanges {
        pr_numbers: Vec<u64>,
        message: String,
    },
    /// Request changes started for a PR (repo_idx, pr_number)
    RequestChangesStart(usize, usize),
    /// Request changes succeeded (repo_idx, pr_number)
    RequestChangesSuccess(usize, usize),
    /// Request changes failed (repo_idx, pr_number, error)
    RequestChangesError(usize, usize, String),

    // Close operations
    /// Request to close selected PRs
    CloseRequest,
    /// Close PRs with a custom message (from confirmation popup)
    ClosePrWithMessage {
        pr_numbers: Vec<u64>,
        message: String,
    },
    /// Close started for a PR (repo_idx, pr_number)
    CloseStart(usize, usize),
    /// Close succeeded (repo_idx, pr_number)
    CloseSuccess(usize, usize),
    /// Close failed (repo_idx, pr_number, error)
    CloseError(usize, usize, String),

    // CI/Build Status actions
    /// Request to rerun failed jobs for the current PR
    RerunFailedJobs,
    /// Rerun started for a workflow run (repo_idx, pr_number, run_id)
    RerunStart(usize, u64, u64),
    /// Rerun succeeded (repo_idx, pr_number, run_id)
    RerunSuccess(usize, u64, u64),
    /// Rerun failed (repo_idx, pr_number, run_id, error)
    RerunError(usize, u64, u64, String),

    // Filters
    /// Cycle through filter presets
    CycleFilter,
    /// Set a specific filter
    SetFilter(PrFilter),
    /// Clear the current filter (show all PRs)
    ClearFilter,
}
