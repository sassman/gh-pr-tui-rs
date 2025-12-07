//! GitHub client trait and cache mode definitions
//!
//! This module defines the core `GitHubClient` trait that all client
//! implementations must satisfy, as well as the `CacheMode` enum for
//! controlling caching behavior.

use crate::types::{
    CheckRun, CheckStatus, CiStatus, MergeMethod, MergeResult, PullRequest, ReviewEvent,
    WorkflowRun,
};
use async_trait::async_trait;

/// Cache behavior mode for GitHub API clients
///
/// Controls how the client interacts with the cache layer.
/// This is set at client construction time, not per-request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CacheMode {
    /// No caching - neither read nor write
    /// Use for mutations or when cache would cause issues
    None,

    /// Write-only - skip cache reads, but write responses to cache
    /// Use for "force refresh" to get fresh data while populating cache
    WriteOnly,

    /// Read-only - read from cache, but don't update it
    /// Use for offline mode or when preserving cache state
    #[allow(dead_code)]
    ReadOnly,

    /// Full caching - read from cache, write to cache
    /// Default behavior for normal operations
    #[default]
    ReadWrite,
}

impl CacheMode {
    /// Should we attempt to read from cache before making API call?
    pub fn should_read(&self) -> bool {
        matches!(self, CacheMode::ReadOnly | CacheMode::ReadWrite)
    }

    /// Should we write API responses to cache?
    pub fn should_write(&self) -> bool {
        matches!(self, CacheMode::WriteOnly | CacheMode::ReadWrite)
    }
}

/// GitHub API client trait
///
/// Defines the interface for interacting with the GitHub API.
/// Implementations can be direct (hitting the API) or decorated
/// with caching, rate limiting, retry logic, etc.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow sharing across
/// async tasks and threads.
///
/// # Example
///
/// ```rust,ignore
/// use gh_client::{GitHubClient, PullRequest};
///
/// async fn list_prs(client: &dyn GitHubClient) -> anyhow::Result<Vec<PullRequest>> {
///     client.fetch_pull_requests("rust-lang", "rust", Some("master")).await
/// }
/// ```
#[async_trait]
pub trait GitHubClient: Send + Sync {
    /// Fetch open pull requests for a repository
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner (user or organization)
    /// * `repo` - Repository name
    /// * `base_branch` - Optional base branch filter (e.g., "main")
    ///
    /// # Returns
    ///
    /// A list of open pull requests, or an error if the API call fails.
    async fn fetch_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        base_branch: Option<&str>,
    ) -> anyhow::Result<Vec<PullRequest>>;

    /// Fetch a single pull request by number
    ///
    /// This returns full PR details including additions/deletions
    /// which are not available in the list endpoint.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `pr_number` - Pull request number
    ///
    /// # Returns
    ///
    /// The pull request details, or an error if not found.
    async fn fetch_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<PullRequest>;

    /// Fetch CI check runs for a specific commit
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `commit_sha` - The commit SHA to get checks for
    ///
    /// # Returns
    ///
    /// A list of check runs for the commit.
    async fn fetch_check_runs(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> anyhow::Result<Vec<CheckRun>>;

    /// Fetch combined commit status
    ///
    /// This uses the legacy Status API which some CI systems still use
    /// (as opposed to the newer Checks API).
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `commit_sha` - The commit SHA to get status for
    ///
    /// # Returns
    ///
    /// Combined status from all status checks.
    async fn fetch_commit_status(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> anyhow::Result<CheckStatus>;

    // === PR Operations ===

    /// Merge a pull request
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `pr_number` - Pull request number
    /// * `merge_method` - How to merge (merge commit, squash, or rebase)
    /// * `commit_title` - Optional custom commit title
    /// * `commit_message` - Optional custom commit message
    ///
    /// # Returns
    ///
    /// Result of the merge operation
    async fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        merge_method: MergeMethod,
        commit_title: Option<&str>,
        commit_message: Option<&str>,
    ) -> anyhow::Result<MergeResult>;

    /// Update a PR's head branch with the latest from base branch (rebase)
    ///
    /// This is equivalent to clicking "Update branch" in the GitHub UI.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `pr_number` - Pull request number
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error on failure
    async fn update_pull_request_branch(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<()>;

    /// Create a review on a pull request
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `pr_number` - Pull request number
    /// * `event` - Review event (approve, request changes, or comment)
    /// * `body` - Optional review comment body
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error on failure
    async fn create_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        event: ReviewEvent,
        body: Option<&str>,
    ) -> anyhow::Result<()>;

    /// Close a pull request without merging
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `pr_number` - Pull request number
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error on failure
    async fn close_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<()>;

    // === CI Operations ===

    /// Rerun failed workflow jobs for a specific run
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `run_id` - Workflow run ID
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error on failure
    async fn rerun_failed_jobs(&self, owner: &str, repo: &str, run_id: u64) -> anyhow::Result<()>;

    /// Fetch workflow runs for a commit
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `head_sha` - The commit SHA to get workflow runs for
    ///
    /// # Returns
    ///
    /// List of workflow runs for the commit
    async fn fetch_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
    ) -> anyhow::Result<Vec<WorkflowRun>>;

    /// Fetch aggregated CI status for a commit
    ///
    /// This fetches all check runs for a commit and aggregates them into
    /// a single status. The aggregation logic is:
    /// - Any failure → Failure
    /// - Any pending (and no failure) → Pending
    /// - All success → Success
    /// - No checks → Unknown
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `head_sha` - The commit SHA to get CI status for
    ///
    /// # Returns
    ///
    /// Aggregated CI status with overall state and counts
    async fn fetch_ci_status(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
    ) -> anyhow::Result<CiStatus>;

    /// Create a review comment on a specific line of a pull request
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `pr_number` - Pull request number
    /// * `commit_id` - The SHA of the commit to comment on (usually head SHA)
    /// * `path` - File path relative to repository root
    /// * `line` - Line number in the file
    /// * `side` - Which side of the diff ("LEFT" for deletions, "RIGHT" for additions)
    /// * `body` - Comment body text
    ///
    /// # Returns
    ///
    /// The GitHub comment ID on success, error on failure
    #[allow(clippy::too_many_arguments)]
    async fn create_review_comment(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        commit_id: &str,
        path: &str,
        line: u32,
        side: &str,
        body: &str,
    ) -> anyhow::Result<u64>;

    /// Delete a review comment
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `comment_id` - The GitHub comment ID to delete
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error on failure
    async fn delete_review_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
    ) -> anyhow::Result<()>;

    /// Fetch review comments for a pull request
    ///
    /// Returns all review comments (line comments) on a PR.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `pr_number` - Pull request number
    ///
    /// # Returns
    ///
    /// List of review comments on the PR
    async fn fetch_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<Vec<crate::types::ReviewComment>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_mode_default() {
        assert_eq!(CacheMode::default(), CacheMode::ReadWrite);
    }

    #[test]
    fn test_cache_mode_should_read() {
        assert!(!CacheMode::None.should_read());
        assert!(!CacheMode::WriteOnly.should_read());
        assert!(CacheMode::ReadOnly.should_read());
        assert!(CacheMode::ReadWrite.should_read());
    }

    #[test]
    fn test_cache_mode_should_write() {
        assert!(!CacheMode::None.should_write());
        assert!(CacheMode::WriteOnly.should_write());
        assert!(!CacheMode::ReadOnly.should_write());
        assert!(CacheMode::ReadWrite.should_write());
    }
}
