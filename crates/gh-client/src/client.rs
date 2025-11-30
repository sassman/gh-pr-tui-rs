//! GitHub client trait and cache mode definitions
//!
//! This module defines the core `GitHubClient` trait that all client
//! implementations must satisfy, as well as the `CacheMode` enum for
//! controlling caching behavior.

use crate::types::{CheckRun, CheckStatus, PullRequest};
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
