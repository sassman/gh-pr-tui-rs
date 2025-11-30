//! Cached GitHub API client (decorator pattern)
//!
//! Wraps any `GitHubClient` implementation to add caching behavior.
//! The cache mode determines whether to read from cache, write to cache, or both.

use crate::client::{CacheMode, GitHubClient};
use crate::types::{CheckRun, CheckStatus, PullRequest};
use async_trait::async_trait;
use gh_api_cache::{ApiCache, CachedResponse};
use log::debug;
use std::sync::{Arc, Mutex};

/// Cached GitHub API client using the decorator pattern
///
/// Wraps an inner `GitHubClient` and adds caching behavior based on the configured
/// `CacheMode`. This allows transparent caching without the caller needing to be
/// aware of the cache.
///
/// # Cache Modes
///
/// - `CacheMode::None` - Pass through to inner client (no caching)
/// - `CacheMode::WriteOnly` - Skip cache reads, but write responses (force refresh)
/// - `CacheMode::ReadOnly` - Read from cache only, don't update cache
/// - `CacheMode::ReadWrite` - Full caching (default, most efficient)
///
/// # Example
///
/// ```rust,ignore
/// use gh_client::{CachedGitHubClient, OctocrabClient, CacheMode};
/// use gh_api_cache::ApiCache;
/// use std::sync::{Arc, Mutex};
///
/// // Create a cached client wrapping the direct client
/// let octocrab = Arc::new(octocrab::Octocrab::builder().build().unwrap());
/// let inner = OctocrabClient::new(octocrab);
/// let cache = Arc::new(Mutex::new(ApiCache::default()));
///
/// let client = CachedGitHubClient::new(inner, cache, CacheMode::ReadWrite);
/// ```
#[derive(Debug, Clone)]
pub struct CachedGitHubClient<C: GitHubClient + Clone> {
    inner: C,
    cache: Arc<Mutex<ApiCache>>,
    mode: CacheMode,
}

impl<C: GitHubClient + Clone> CachedGitHubClient<C> {
    /// Create a new cached client
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner client to delegate API calls to
    /// * `cache` - Shared cache instance
    /// * `mode` - Cache behavior mode
    pub fn new(inner: C, cache: Arc<Mutex<ApiCache>>, mode: CacheMode) -> Self {
        Self { inner, cache, mode }
    }

    /// Get the current cache mode
    pub fn cache_mode(&self) -> CacheMode {
        self.mode
    }

    /// Create a new client with a different cache mode
    ///
    /// This is useful for creating a "force refresh" client without
    /// constructing a new inner client.
    pub fn with_mode(&self, mode: CacheMode) -> CachedGitHubClient<C>
    where
        C: Clone,
    {
        CachedGitHubClient {
            inner: self.inner.clone(),
            cache: Arc::clone(&self.cache),
            mode,
        }
    }

    /// Try to get data from cache
    fn try_cache_get(&self, method: &str, url: &str, params: &[(&str, &str)]) -> Option<String> {
        if !self.mode.should_read() {
            return None;
        }

        let cache = self.cache.lock().unwrap();
        cache.get(method, url, params).map(|r| r.body)
    }

    /// Write data to cache
    fn cache_set(&self, method: &str, url: &str, params: &[(&str, &str)], body: &str) {
        if !self.mode.should_write() {
            return;
        }

        let response = CachedResponse {
            body: body.to_string(),
            etag: None,
            status_code: 200,
        };

        let mut cache = self.cache.lock().unwrap();
        if let Err(e) = cache.set(method, url, params, &response) {
            debug!("Failed to write to cache: {}", e);
        }
    }
}

#[async_trait]
impl<C: GitHubClient + Clone> GitHubClient for CachedGitHubClient<C> {
    async fn fetch_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        base_branch: Option<&str>,
    ) -> anyhow::Result<Vec<PullRequest>> {
        let url = format!("/repos/{}/{}/pulls", owner, repo);
        let params: Vec<(&str, &str)> = if let Some(branch) = base_branch {
            vec![("state", "open"), ("head", branch)]
        } else {
            vec![("state", "open")]
        };

        // Try cache first
        if let Some(cached_body) = self.try_cache_get("GET", &url, &params) {
            match serde_json::from_str::<Vec<PullRequest>>(&cached_body) {
                Ok(prs) => {
                    debug!(
                        "Cache HIT for {}/{}: {} PRs",
                        owner,
                        repo,
                        prs.len()
                    );
                    return Ok(prs);
                }
                Err(e) => {
                    debug!("Failed to parse cached PRs: {}", e);
                    // Fall through to fetch fresh data
                }
            }
        }

        // Fetch from API
        let prs = self.inner.fetch_pull_requests(owner, repo, base_branch).await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&prs) {
            self.cache_set("GET", &url, &params, &json);
        }

        Ok(prs)
    }

    async fn fetch_check_runs(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> anyhow::Result<Vec<CheckRun>> {
        let url = format!("/repos/{}/{}/commits/{}/check-runs", owner, repo, commit_sha);
        let params: &[(&str, &str)] = &[];

        // Try cache first
        if let Some(cached_body) = self.try_cache_get("GET", &url, params) {
            match serde_json::from_str::<Vec<CheckRun>>(&cached_body) {
                Ok(runs) => {
                    debug!(
                        "Cache HIT for {}/{} @ {}: {} check runs",
                        owner,
                        repo,
                        commit_sha,
                        runs.len()
                    );
                    return Ok(runs);
                }
                Err(e) => {
                    debug!("Failed to parse cached check runs: {}", e);
                }
            }
        }

        // Fetch from API
        let runs = self.inner.fetch_check_runs(owner, repo, commit_sha).await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&runs) {
            self.cache_set("GET", &url, params, &json);
        }

        Ok(runs)
    }

    async fn fetch_commit_status(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> anyhow::Result<CheckStatus> {
        let url = format!("/repos/{}/{}/commits/{}/status", owner, repo, commit_sha);
        let params: &[(&str, &str)] = &[];

        // Try cache first
        if let Some(cached_body) = self.try_cache_get("GET", &url, params) {
            match serde_json::from_str::<CheckStatus>(&cached_body) {
                Ok(status) => {
                    debug!(
                        "Cache HIT for {}/{} @ {}: {:?}",
                        owner, repo, commit_sha, status.state
                    );
                    return Ok(status);
                }
                Err(e) => {
                    debug!("Failed to parse cached commit status: {}", e);
                }
            }
        }

        // Fetch from API
        let status = self.inner.fetch_commit_status(owner, repo, commit_sha).await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&status) {
            self.cache_set("GET", &url, params, &json);
        }

        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CheckState, MergeableState};
    use chrono::Utc;

    /// Mock client for testing
    #[derive(Debug, Clone)]
    struct MockClient {
        prs: Vec<PullRequest>,
        call_count: Arc<Mutex<usize>>,
    }

    impl MockClient {
        fn new(prs: Vec<PullRequest>) -> Self {
            Self {
                prs,
                call_count: Arc::new(Mutex::new(0)),
            }
        }

        fn call_count(&self) -> usize {
            *self.call_count.lock().unwrap()
        }
    }

    #[async_trait]
    impl GitHubClient for MockClient {
        async fn fetch_pull_requests(
            &self,
            _owner: &str,
            _repo: &str,
            _base_branch: Option<&str>,
        ) -> anyhow::Result<Vec<PullRequest>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(self.prs.clone())
        }

        async fn fetch_check_runs(
            &self,
            _owner: &str,
            _repo: &str,
            _commit_sha: &str,
        ) -> anyhow::Result<Vec<CheckRun>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(vec![])
        }

        async fn fetch_commit_status(
            &self,
            _owner: &str,
            _repo: &str,
            _commit_sha: &str,
        ) -> anyhow::Result<CheckStatus> {
            *self.call_count.lock().unwrap() += 1;
            Ok(CheckStatus {
                state: CheckState::Success,
                total_count: 0,
                statuses: vec![],
            })
        }
    }

    fn create_test_pr(number: u64) -> PullRequest {
        PullRequest {
            number,
            title: format!("Test PR {}", number),
            body: None,
            author: "testuser".to_string(),
            comments: 0,
            head_sha: "abc123".to_string(),
            base_branch: "main".to_string(),
            head_branch: "feature".to_string(),
            mergeable: Some(true),
            mergeable_state: Some(MergeableState::Clean),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            html_url: "https://github.com/test/repo/pull/1".to_string(),
        }
    }

    #[tokio::test]
    async fn test_cache_mode_none_skips_cache() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let client = CachedGitHubClient::new(mock.clone(), cache, CacheMode::None);

        // First call
        let prs1 = client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(prs1.len(), 1);
        assert_eq!(mock.call_count(), 1);

        // Second call - should NOT use cache (mode is None)
        let prs2 = client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(prs2.len(), 1);
        assert_eq!(mock.call_count(), 2); // Called again, not cached
    }

    #[tokio::test]
    async fn test_cache_mode_read_write_caches() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let client = CachedGitHubClient::new(mock.clone(), cache, CacheMode::ReadWrite);

        // First call - cache miss, calls mock
        let prs1 = client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(prs1.len(), 1);
        assert_eq!(mock.call_count(), 1);

        // Second call - should use cache
        let prs2 = client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(prs2.len(), 1);
        assert_eq!(mock.call_count(), 1); // Still 1, used cache
    }

    #[tokio::test]
    async fn test_cache_mode_write_only_skips_read() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let client = CachedGitHubClient::new(mock.clone(), cache.clone(), CacheMode::WriteOnly);

        // First call - writes to cache
        let prs1 = client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(prs1.len(), 1);
        assert_eq!(mock.call_count(), 1);

        // Second call - should NOT read from cache (WriteOnly mode)
        let prs2 = client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(prs2.len(), 1);
        assert_eq!(mock.call_count(), 2); // Called again

        // But cache should have the data (verify with ReadWrite mode)
        let read_client = CachedGitHubClient::new(mock.clone(), cache, CacheMode::ReadWrite);
        let prs3 = read_client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(prs3.len(), 1);
        assert_eq!(mock.call_count(), 2); // Still 2, used cache
    }

    #[tokio::test]
    async fn test_cache_mode_read_only_skips_write() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));

        // First, populate cache with ReadWrite
        let write_client =
            CachedGitHubClient::new(mock.clone(), cache.clone(), CacheMode::ReadWrite);
        write_client.fetch_pull_requests("owner", "repo", None).await.unwrap();
        assert_eq!(mock.call_count(), 1);

        // Create new mock with different data
        let mock2 = MockClient::new(vec![create_test_pr(2)]);

        // ReadOnly client should read from cache
        let read_client = CachedGitHubClient::new(mock2.clone(), cache, CacheMode::ReadOnly);
        let prs = read_client.fetch_pull_requests("owner", "repo", None).await.unwrap();

        // Should get cached data (PR #1), not new mock data (PR #2)
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 1);
        assert_eq!(mock2.call_count(), 0); // Never called, used cache
    }

    #[tokio::test]
    async fn test_with_mode_creates_new_client() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let client = CachedGitHubClient::new(mock, cache, CacheMode::ReadWrite);

        assert_eq!(client.cache_mode(), CacheMode::ReadWrite);

        let force_refresh = client.with_mode(CacheMode::WriteOnly);
        assert_eq!(force_refresh.cache_mode(), CacheMode::WriteOnly);

        // Original client unchanged
        assert_eq!(client.cache_mode(), CacheMode::ReadWrite);
    }
}
