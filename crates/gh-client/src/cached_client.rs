//! Cached GitHub API client (decorator pattern)
//!
//! Wraps any `GitHubClient` implementation to add caching behavior.
//! The cache mode determines whether to read from cache, write to cache, or both.

use crate::client::{CacheMode, GitHubClient};
use crate::types::{
    CheckRun, CheckStatus, CiStatus, MergeMethod, MergeResult, PullRequest, ReviewComment,
    ReviewEvent, WorkflowRun,
};
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

    /// Get a reference to the inner client
    ///
    /// This allows access to client-specific methods not covered by GitHubClient trait.
    pub fn inner(&self) -> &C {
        &self.inner
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

    /// Invalidate comment-related cache entries for a repository
    ///
    /// Called after mutations (POST/DELETE) to ensure comment lists are refreshed.
    /// Uses pattern `/repos/{owner}/{repo}/pulls/` which matches:
    /// - GET `/repos/{owner}/{repo}/pulls/{pr}/comments` (comment lists)
    /// - But NOT `/repos/{owner}/{repo}/pulls?state=open` (PR lists)
    fn cache_invalidate_comments(&self, owner: &str, repo: &str) {
        // Pattern with trailing slash matches PR-specific endpoints (comments, reviews, etc.)
        // but not the PR list endpoint (/pulls?state=open)
        let pattern = format!("/repos/{}/{}/pulls/", owner, repo);
        debug!(
            "Cache invalidation for comments: invalidating pattern '{}'",
            pattern
        );
        let mut cache = self.cache.lock().unwrap();
        cache.invalidate_pattern(&pattern);
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
                Ok(mut prs) => {
                    // Always sort for stable ordering (descending by PR number)
                    prs.sort_by(|a, b| b.number.cmp(&a.number));
                    prs.dedup_by_key(|pr| pr.number);
                    debug!("Cache HIT for {}/{}: {} PRs", owner, repo, prs.len());
                    return Ok(prs);
                }
                Err(e) => {
                    debug!("Failed to parse cached PRs: {}", e);
                    // Fall through to fetch fresh data
                }
            }
        }

        // Fetch from API
        let prs = self
            .inner
            .fetch_pull_requests(owner, repo, base_branch)
            .await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&prs) {
            self.cache_set("GET", &url, &params, &json);
        }

        Ok(prs)
    }

    async fn fetch_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<PullRequest> {
        let url = format!("/repos/{}/{}/pulls/{}", owner, repo, pr_number);
        let params: &[(&str, &str)] = &[];

        // Try cache first
        if let Some(cached_body) = self.try_cache_get("GET", &url, params) {
            match serde_json::from_str::<PullRequest>(&cached_body) {
                Ok(pr) => {
                    debug!("Cache HIT for PR #{} in {}/{}", pr_number, owner, repo);
                    return Ok(pr);
                }
                Err(e) => {
                    debug!("Cache parse error for PR #{}: {}", pr_number, e);
                }
            }
        }

        // Cache miss - fetch from API
        debug!("Cache MISS for PR #{} in {}/{}", pr_number, owner, repo);
        let pr = self.inner.fetch_pull_request(owner, repo, pr_number).await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&pr) {
            self.cache_set("GET", &url, &params, &json);
        }

        Ok(pr)
    }

    async fn fetch_check_runs(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> anyhow::Result<Vec<CheckRun>> {
        let url = format!(
            "/repos/{}/{}/commits/{}/check-runs",
            owner, repo, commit_sha
        );
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
        let status = self
            .inner
            .fetch_commit_status(owner, repo, commit_sha)
            .await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&status) {
            self.cache_set("GET", &url, params, &json);
        }

        Ok(status)
    }

    // PR operations are mutations - pass through to inner client without caching

    async fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        merge_method: MergeMethod,
        commit_title: Option<&str>,
        commit_message: Option<&str>,
    ) -> anyhow::Result<MergeResult> {
        // Mutations are never cached - pass through directly
        self.inner
            .merge_pull_request(
                owner,
                repo,
                pr_number,
                merge_method,
                commit_title,
                commit_message,
            )
            .await
    }

    async fn update_pull_request_branch(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<()> {
        // Mutations are never cached - pass through directly
        self.inner
            .update_pull_request_branch(owner, repo, pr_number)
            .await
    }

    async fn create_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        event: ReviewEvent,
        body: Option<&str>,
    ) -> anyhow::Result<()> {
        // Mutations are never cached - pass through directly
        self.inner
            .create_review(owner, repo, pr_number, event, body)
            .await
    }

    async fn close_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<()> {
        // Mutations are never cached - pass through directly
        self.inner.close_pull_request(owner, repo, pr_number).await
    }

    async fn rerun_failed_jobs(&self, owner: &str, repo: &str, run_id: u64) -> anyhow::Result<()> {
        // Mutations are never cached - pass through directly
        self.inner.rerun_failed_jobs(owner, repo, run_id).await
    }

    async fn fetch_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
    ) -> anyhow::Result<Vec<WorkflowRun>> {
        let url = format!("/repos/{}/{}/actions/runs", owner, repo);
        let params: &[(&str, &str)] = &[("head_sha", head_sha)];

        // Try cache first
        if let Some(cached_body) = self.try_cache_get("GET", &url, params) {
            match serde_json::from_str::<Vec<WorkflowRun>>(&cached_body) {
                Ok(runs) => {
                    debug!(
                        "Cache HIT for {}/{} @ {}: {} workflow runs",
                        owner,
                        repo,
                        head_sha,
                        runs.len()
                    );
                    return Ok(runs);
                }
                Err(e) => {
                    debug!("Failed to parse cached workflow runs: {}", e);
                }
            }
        }

        // Fetch from API
        let runs = self
            .inner
            .fetch_workflow_runs(owner, repo, head_sha)
            .await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&runs) {
            self.cache_set("GET", &url, params, &json);
        }

        Ok(runs)
    }

    async fn fetch_ci_status(
        &self,
        owner: &str,
        repo: &str,
        head_sha: &str,
    ) -> anyhow::Result<CiStatus> {
        let url = format!("/repos/{}/{}/commits/{}/check-runs", owner, repo, head_sha);
        let params: &[(&str, &str)] = &[];

        // Try cache first
        if let Some(cached_body) = self.try_cache_get("GET", &url, params) {
            match serde_json::from_str::<CiStatus>(&cached_body) {
                Ok(status) => {
                    debug!(
                        "Cache HIT for CI status {}/{} @ {}: {:?}",
                        owner, repo, head_sha, status.state
                    );
                    return Ok(status);
                }
                Err(e) => {
                    debug!("Failed to parse cached CI status: {}", e);
                }
            }
        }

        // Fetch from API
        let status = self.inner.fetch_ci_status(owner, repo, head_sha).await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&status) {
            self.cache_set("GET", &url, params, &json);
        }

        Ok(status)
    }

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
    ) -> anyhow::Result<u64> {
        // Execute the create
        let result = self
            .inner
            .create_review_comment(owner, repo, pr_number, commit_id, path, line, side, body)
            .await;

        // On success, invalidate cached comments for this repo
        if result.is_ok() {
            self.cache_invalidate_comments(owner, repo);
        }

        result
    }

    async fn delete_review_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
    ) -> anyhow::Result<()> {
        // Execute the delete
        let result = self
            .inner
            .delete_review_comment(owner, repo, comment_id)
            .await;

        // On success, invalidate cached comments for this repo
        if result.is_ok() {
            self.cache_invalidate_comments(owner, repo);
        }

        result
    }

    async fn fetch_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> anyhow::Result<Vec<ReviewComment>> {
        let url = format!("/repos/{}/{}/pulls/{}/comments", owner, repo, pr_number);
        let params: &[(&str, &str)] = &[];

        // Try cache first
        if let Some(cached_body) = self.try_cache_get("GET", &url, params) {
            match serde_json::from_str::<Vec<ReviewComment>>(&cached_body) {
                Ok(comments) => {
                    debug!(
                        "Cache HIT for {}/{} PR #{} comments: {} comments",
                        owner,
                        repo,
                        pr_number,
                        comments.len()
                    );
                    return Ok(comments);
                }
                Err(e) => {
                    debug!("Failed to parse cached comments: {}", e);
                    // Fall through to fetch fresh data
                }
            }
        }

        // Fetch from API
        let comments = self
            .inner
            .fetch_review_comments(owner, repo, pr_number)
            .await?;

        // Cache the result
        if let Ok(json) = serde_json::to_string(&comments) {
            self.cache_set("GET", &url, params, &json);
        }

        Ok(comments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CheckState, CiState, MergeableState};
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

        async fn fetch_pull_request(
            &self,
            _owner: &str,
            _repo: &str,
            pr_number: u64,
        ) -> anyhow::Result<PullRequest> {
            *self.call_count.lock().unwrap() += 1;
            self.prs
                .iter()
                .find(|pr| pr.number == pr_number)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("PR not found"))
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

        async fn merge_pull_request(
            &self,
            _owner: &str,
            _repo: &str,
            _pr_number: u64,
            _merge_method: MergeMethod,
            _commit_title: Option<&str>,
            _commit_message: Option<&str>,
        ) -> anyhow::Result<MergeResult> {
            *self.call_count.lock().unwrap() += 1;
            Ok(MergeResult {
                merged: true,
                sha: Some("abc123".to_string()),
                message: "Merged".to_string(),
            })
        }

        async fn update_pull_request_branch(
            &self,
            _owner: &str,
            _repo: &str,
            _pr_number: u64,
        ) -> anyhow::Result<()> {
            *self.call_count.lock().unwrap() += 1;
            Ok(())
        }

        async fn create_review(
            &self,
            _owner: &str,
            _repo: &str,
            _pr_number: u64,
            _event: ReviewEvent,
            _body: Option<&str>,
        ) -> anyhow::Result<()> {
            *self.call_count.lock().unwrap() += 1;
            Ok(())
        }

        async fn close_pull_request(
            &self,
            _owner: &str,
            _repo: &str,
            _pr_number: u64,
        ) -> anyhow::Result<()> {
            *self.call_count.lock().unwrap() += 1;
            Ok(())
        }

        async fn rerun_failed_jobs(
            &self,
            _owner: &str,
            _repo: &str,
            _run_id: u64,
        ) -> anyhow::Result<()> {
            *self.call_count.lock().unwrap() += 1;
            Ok(())
        }

        async fn fetch_workflow_runs(
            &self,
            _owner: &str,
            _repo: &str,
            _head_sha: &str,
        ) -> anyhow::Result<Vec<WorkflowRun>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(vec![])
        }

        async fn fetch_ci_status(
            &self,
            _owner: &str,
            _repo: &str,
            _head_sha: &str,
        ) -> anyhow::Result<CiStatus> {
            *self.call_count.lock().unwrap() += 1;
            Ok(CiStatus {
                state: CiState::Success,
                total_checks: 0,
                passed: 0,
                failed: 0,
                pending: 0,
            })
        }

        async fn create_review_comment(
            &self,
            _owner: &str,
            _repo: &str,
            _pr_number: u64,
            _commit_id: &str,
            _path: &str,
            _line: u32,
            _side: &str,
            _body: &str,
        ) -> anyhow::Result<u64> {
            *self.call_count.lock().unwrap() += 1;
            Ok(12345) // Mock comment ID
        }

        async fn delete_review_comment(
            &self,
            _owner: &str,
            _repo: &str,
            _comment_id: u64,
        ) -> anyhow::Result<()> {
            *self.call_count.lock().unwrap() += 1;
            Ok(())
        }

        async fn fetch_review_comments(
            &self,
            _owner: &str,
            _repo: &str,
            _pr_number: u64,
        ) -> anyhow::Result<Vec<ReviewComment>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(vec![]) // Empty list by default
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
            additions: 100,
            deletions: 50,
        }
    }

    #[tokio::test]
    async fn test_cache_mode_none_skips_cache() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let client = CachedGitHubClient::new(mock.clone(), cache, CacheMode::None);

        // First call
        let prs1 = client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(prs1.len(), 1);
        assert_eq!(mock.call_count(), 1);

        // Second call - should NOT use cache (mode is None)
        let prs2 = client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(prs2.len(), 1);
        assert_eq!(mock.call_count(), 2); // Called again, not cached
    }

    #[tokio::test]
    async fn test_cache_mode_read_write_caches() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let client = CachedGitHubClient::new(mock.clone(), cache, CacheMode::ReadWrite);

        // First call - cache miss, calls mock
        let prs1 = client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(prs1.len(), 1);
        assert_eq!(mock.call_count(), 1);

        // Second call - should use cache
        let prs2 = client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(prs2.len(), 1);
        assert_eq!(mock.call_count(), 1); // Still 1, used cache
    }

    #[tokio::test]
    async fn test_cache_mode_write_only_skips_read() {
        let mock = MockClient::new(vec![create_test_pr(1)]);
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let client = CachedGitHubClient::new(mock.clone(), cache.clone(), CacheMode::WriteOnly);

        // First call - writes to cache
        let prs1 = client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(prs1.len(), 1);
        assert_eq!(mock.call_count(), 1);

        // Second call - should NOT read from cache (WriteOnly mode)
        let prs2 = client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(prs2.len(), 1);
        assert_eq!(mock.call_count(), 2); // Called again

        // But cache should have the data (verify with ReadWrite mode)
        let read_client = CachedGitHubClient::new(mock.clone(), cache, CacheMode::ReadWrite);
        let prs3 = read_client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
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
        write_client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();
        assert_eq!(mock.call_count(), 1);

        // Create new mock with different data
        let mock2 = MockClient::new(vec![create_test_pr(2)]);

        // ReadOnly client should read from cache
        let read_client = CachedGitHubClient::new(mock2.clone(), cache, CacheMode::ReadOnly);
        let prs = read_client
            .fetch_pull_requests("owner", "repo", None)
            .await
            .unwrap();

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
