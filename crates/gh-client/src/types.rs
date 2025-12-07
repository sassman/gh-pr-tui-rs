//! GitHub API data transfer objects
//!
//! These types represent the data returned from the GitHub API.
//! They are intentionally separate from application domain models
//! to keep this crate pure and reusable.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A pull request from the GitHub API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// PR number (e.g., 123)
    pub number: u64,

    /// PR title
    pub title: String,

    /// PR body/description
    pub body: Option<String>,

    /// Author's GitHub username
    pub author: String,

    /// Number of comments on the PR
    pub comments: u64,

    /// HEAD commit SHA
    pub head_sha: String,

    /// Base branch name (e.g., "main")
    pub base_branch: String,

    /// HEAD branch name (e.g., "feature/foo")
    pub head_branch: String,

    /// Whether the PR is mergeable (null if not yet computed by GitHub)
    pub mergeable: Option<bool>,

    /// Mergeable state from GitHub
    pub mergeable_state: Option<MergeableState>,

    /// When the PR was created
    pub created_at: DateTime<Utc>,

    /// When the PR was last updated
    pub updated_at: DateTime<Utc>,

    /// PR URL for opening in browser
    pub html_url: String,

    /// Number of lines added
    pub additions: u64,

    /// Number of lines deleted
    pub deletions: u64,
}

/// Mergeable state as reported by GitHub
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeableState {
    /// The merge is clean
    Clean,
    /// The head branch is behind the base branch
    Behind,
    /// The merge has conflicts
    Dirty,
    /// The merge is blocked (e.g., by required reviews)
    Blocked,
    /// CI checks are failing or pending
    Unstable,
    /// State is unknown or not yet computed
    #[default]
    Unknown,
}

/// A CI check run from the GitHub API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRun {
    /// Check run ID
    pub id: u64,

    /// Name of the check (e.g., "build", "test")
    pub name: String,

    /// Current status
    pub status: CheckRunStatus,

    /// Conclusion (only set when status is Completed)
    pub conclusion: Option<CheckConclusion>,

    /// URL to the check run details
    pub details_url: Option<String>,

    /// When the check started
    pub started_at: Option<DateTime<Utc>>,

    /// When the check completed
    pub completed_at: Option<DateTime<Utc>>,
}

/// Status of a check run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunStatus {
    /// Check is queued
    Queued,
    /// Check is in progress
    InProgress,
    /// Check has completed
    Completed,
}

/// Conclusion of a completed check run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion {
    /// Check passed
    Success,
    /// Check failed
    Failure,
    /// Check was neutral (neither success nor failure)
    Neutral,
    /// Check was cancelled
    Cancelled,
    /// Check was skipped
    Skipped,
    /// Check timed out
    TimedOut,
    /// Action is required from the user
    ActionRequired,
    /// Check is stale (superseded by newer run)
    Stale,
}

/// Combined commit status from the GitHub API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckStatus {
    /// Overall state combining all statuses
    pub state: CheckState,

    /// Total number of status checks
    pub total_count: u64,

    /// Individual statuses
    pub statuses: Vec<CommitStatus>,
}

/// Overall state of combined commit status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    /// All checks passed
    Success,
    /// At least one check is pending
    Pending,
    /// At least one check failed
    Failure,
    /// Error retrieving status
    Error,
}

/// Individual commit status (from the Status API, not Checks API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStatus {
    /// Status context (e.g., "ci/circleci")
    pub context: String,

    /// Current state
    pub state: CheckState,

    /// Description of the status
    pub description: Option<String>,

    /// URL for more details
    pub target_url: Option<String>,
}

/// Merge method for pull requests
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    /// Create a merge commit
    Merge,
    /// Squash all commits into one
    #[default]
    Squash,
    /// Rebase commits onto the base branch
    Rebase,
}

/// Result of a merge operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    /// Whether the merge was successful
    pub merged: bool,
    /// Commit SHA of the merge commit (if successful)
    pub sha: Option<String>,
    /// Message from the merge operation
    pub message: String,
}

/// Review event type for PR reviews
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewEvent {
    /// Approve the PR
    Approve,
    /// Request changes
    RequestChanges,
    /// Comment only (no approval/rejection)
    Comment,
}

/// A GitHub Actions workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    /// Workflow run ID
    pub id: u64,
    /// Name of the workflow
    pub name: String,
    /// Status of the run
    pub status: WorkflowRunStatus,
    /// Conclusion (only set when completed)
    pub conclusion: Option<WorkflowRunConclusion>,
    /// HEAD SHA the workflow ran on
    pub head_sha: String,
    /// URL to view the workflow run
    pub html_url: String,
    /// When the run was created
    pub created_at: DateTime<Utc>,
    /// When the run was last updated
    pub updated_at: DateTime<Utc>,
}

/// Status of a workflow run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    /// Workflow is queued
    Queued,
    /// Workflow is waiting
    Waiting,
    /// Workflow is in progress
    InProgress,
    /// Workflow has completed
    Completed,
    /// Workflow is pending
    Pending,
}

/// Conclusion of a completed workflow run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunConclusion {
    /// Workflow succeeded
    Success,
    /// Workflow failed
    Failure,
    /// Workflow was neutral
    Neutral,
    /// Workflow was cancelled
    Cancelled,
    /// Workflow was skipped
    Skipped,
    /// Workflow timed out
    TimedOut,
    /// Action required
    ActionRequired,
    /// Workflow is stale
    Stale,
}

/// Aggregated CI status from check runs
///
/// This represents the combined status of all CI check runs for a commit,
/// aggregated into a single overall state with counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiStatus {
    /// Overall CI state (aggregated from all check runs)
    pub state: CiState,
    /// Total number of check runs
    pub total_checks: usize,
    /// Number of passed checks
    pub passed: usize,
    /// Number of failed checks
    pub failed: usize,
    /// Number of pending/in-progress checks
    pub pending: usize,
}

/// Aggregated CI state
///
/// Represents the overall state of CI for a commit:
/// - Any failure → Failure
/// - Any pending (and no failure) → Pending
/// - All success → Success
/// - No checks → Unknown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiState {
    /// All checks passed
    Success,
    /// At least one check failed
    Failure,
    /// At least one check is pending/in-progress (no failures)
    Pending,
    /// No checks found or status unknown
    Unknown,
}

/// A review comment on a pull request
///
/// Represents a comment on a specific line in a PR diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// GitHub comment ID
    pub id: u64,
    /// File path the comment is on
    pub path: String,
    /// Line number in the diff (for new code)
    pub line: Option<u32>,
    /// Original line number (for old code)
    pub original_line: Option<u32>,
    /// Which side of the diff: "LEFT" (deletions) or "RIGHT" (additions)
    pub side: Option<String>,
    /// Comment body text
    pub body: String,
    /// Author's GitHub username
    pub author: String,
    /// When the comment was created
    pub created_at: DateTime<Utc>,
    /// When the comment was last updated
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mergeable_state_default() {
        assert_eq!(MergeableState::default(), MergeableState::Unknown);
    }

    #[test]
    fn test_pull_request_serialization() {
        let pr = PullRequest {
            number: 42,
            title: "Test PR".to_string(),
            body: Some("Description".to_string()),
            author: "testuser".to_string(),
            comments: 5,
            head_sha: "abc123".to_string(),
            base_branch: "main".to_string(),
            head_branch: "feature/test".to_string(),
            mergeable: Some(true),
            mergeable_state: Some(MergeableState::Clean),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            html_url: "https://github.com/owner/repo/pull/42".to_string(),
            additions: 100,
            deletions: 50,
        };

        let json = serde_json::to_string(&pr).unwrap();
        let deserialized: PullRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.number, 42);
        assert_eq!(deserialized.title, "Test PR");
        assert_eq!(deserialized.author, "testuser");
    }

    #[test]
    fn test_check_run_serialization() {
        let check = CheckRun {
            id: 1,
            name: "build".to_string(),
            status: CheckRunStatus::Completed,
            conclusion: Some(CheckConclusion::Success),
            details_url: Some("https://example.com".to_string()),
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&check).unwrap();
        let deserialized: CheckRun = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "build");
        assert_eq!(deserialized.status, CheckRunStatus::Completed);
        assert_eq!(deserialized.conclusion, Some(CheckConclusion::Success));
    }

    #[test]
    fn test_mergeable_state_serde() {
        let states = vec![
            (MergeableState::Clean, "\"clean\""),
            (MergeableState::Behind, "\"behind\""),
            (MergeableState::Dirty, "\"dirty\""),
            (MergeableState::Blocked, "\"blocked\""),
            (MergeableState::Unstable, "\"unstable\""),
            (MergeableState::Unknown, "\"unknown\""),
        ];

        for (state, expected_json) in states {
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, expected_json);

            let deserialized: MergeableState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, state);
        }
    }
}
