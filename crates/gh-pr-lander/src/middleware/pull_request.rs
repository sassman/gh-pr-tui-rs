//! Pull Request Middleware
//!
//! Handles side effects for loading Pull Requests from GitHub:
//! - Initializes GitHub client on BootstrapStart
//! - Triggers PR loading when repositories are added
//! - Makes API calls to fetch PRs (with caching support)
//! - Dispatches PrLoaded/PrLoadError actions with results
//!
//! # Caching
//!
//! This middleware uses the `gh-client` crate's decorator pattern for caching:
//! - Normal loads use `CacheMode::ReadWrite` (read from cache, write to cache)
//! - Force refresh uses `CacheMode::WriteOnly` (skip cache, but update it)
//!
//! The caching is transparent to the rest of the application - no boolean flags needed.

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::domain_models::{MergeableStatus, Pr};
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_client::{
    octocrab::Octocrab, ApiCache, CacheMode, CachedGitHubClient, GitHubClient, OctocrabClient,
    PullRequest,
};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

/// Middleware for loading Pull Requests from GitHub
pub struct PullRequestMiddleware {
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// Cached GitHub client (initialized on BootstrapStart)
    client: Option<CachedGitHubClient<OctocrabClient>>,
    /// Shared cache instance
    cache: Arc<Mutex<ApiCache>>,
}

impl PullRequestMiddleware {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");

        // Initialize cache from config path
        let cache_file = gh_pr_config::get_cache_file_path()
            .unwrap_or_else(|_| std::env::temp_dir().join("gh-api-cache.json"));
        let cache = Arc::new(Mutex::new(
            ApiCache::new(cache_file).unwrap_or_default(),
        ));

        Self {
            runtime,
            client: None,
            cache,
        }
    }

    /// Initialize the GitHub client with caching
    fn initialize_client(&mut self) {
        let cache = Arc::clone(&self.cache);
        let result = self.runtime.block_on(async { init_client(cache).await });

        match result {
            Ok(client) => {
                log::info!("PullRequestMiddleware: GitHub client initialized with caching");
                self.client = Some(client);
            }
            Err(e) => {
                log::warn!(
                    "PullRequestMiddleware: GitHub client not initialized: {}",
                    e
                );
            }
        }
    }

    /// Get a client configured for force refresh (WriteOnly mode)
    fn force_refresh_client(&self) -> Option<CachedGitHubClient<OctocrabClient>> {
        self.client
            .as_ref()
            .map(|c| c.with_mode(CacheMode::WriteOnly))
    }
}

impl Default for PullRequestMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for PullRequestMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Initialize client on bootstrap
            Action::BootstrapStart => {
                self.initialize_client();
                true // Let action pass through
            }

            // When repositories are added in bulk, start loading PRs for each
            Action::RepositoryAddBulk(repos) => {
                log::info!(
                    "RepositoryAddBulk received with {} repos, client initialized: {}",
                    repos.len(),
                    self.client.is_some()
                );

                if self.client.is_none() {
                    log::warn!("Cannot load PRs: GitHub client not initialized");
                    return true;
                }

                // Calculate starting index (after existing repos)
                let start_idx = state.main_view.repositories.len();
                log::info!(
                    "Current repos in state: {}, will dispatch PrLoadStart for indices {}..{}",
                    start_idx,
                    start_idx,
                    start_idx + repos.len()
                );

                // Dispatch PrLoadStart for each new repository
                for (i, _repo) in repos.iter().enumerate() {
                    let repo_idx = start_idx + i;
                    dispatcher.dispatch(Action::PrLoadStart(repo_idx));
                }

                true // Let action pass through to reducer
            }

            // When a single repository is added via confirm
            Action::AddRepoConfirm => {
                if self.client.is_none() {
                    log::warn!("Cannot load PRs: GitHub client not initialized");
                    return true;
                }

                if state.add_repo_form.is_valid() {
                    // The new repo will be at the end of the list
                    let repo_idx = state.main_view.repositories.len();
                    dispatcher.dispatch(Action::PrLoadStart(repo_idx));
                }

                true // Let action pass through to reducer
            }

            // Handle PR load start - actually fetch the PRs
            Action::PrLoadStart(repo_idx) => {
                self.handle_pr_load(*repo_idx, state, dispatcher, false)
            }

            // Handle PR refresh request (force refresh - bypass cache)
            Action::PrRefresh => {
                if self.client.is_none() {
                    log::warn!("Cannot refresh PRs: GitHub client not initialized");
                    return true;
                }

                let repo_idx = state.main_view.selected_repository;
                self.handle_pr_load(repo_idx, state, dispatcher, true)
            }

            _ => true, // Pass through all other actions
        }
    }
}

impl PullRequestMiddleware {
    /// Handle loading PRs for a repository
    ///
    /// # Arguments
    /// * `repo_idx` - Index of the repository to load
    /// * `state` - Current app state
    /// * `dispatcher` - Action dispatcher
    /// * `force_refresh` - If true, bypass cache (but still write to it)
    fn handle_pr_load(
        &self,
        repo_idx: usize,
        state: &AppState,
        dispatcher: &Dispatcher,
        force_refresh: bool,
    ) -> bool {
        log::info!(
            "PrLoad({}) received, repos in state: {}, force_refresh: {}",
            repo_idx,
            state.main_view.repositories.len(),
            force_refresh
        );

        let client = if force_refresh {
            let Some(client) = self.force_refresh_client() else {
                log::error!("PrLoad: client not initialized");
                dispatcher.dispatch(Action::PrLoadError(
                    repo_idx,
                    "GitHub client not initialized".to_string(),
                ));
                return true;
            };
            client
        } else {
            let Some(client) = self.client.clone() else {
                log::error!("PrLoad: client not initialized");
                dispatcher.dispatch(Action::PrLoadError(
                    repo_idx,
                    "GitHub client not initialized".to_string(),
                ));
                return true;
            };
            client
        };

        // Get the repository at this index
        let Some(repo) = state.main_view.repositories.get(repo_idx) else {
            log::warn!(
                "PrLoad: Repository at index {} not found (state has {} repos), will retry",
                repo_idx,
                state.main_view.repositories.len()
            );
            return true;
        };

        log::info!(
            "PrLoad: Found repo at index {}: {}/{}",
            repo_idx,
            repo.org,
            repo.repo
        );

        let org = repo.org.clone();
        let repo_name = repo.repo.clone();
        let base_branch = Some(repo.branch.clone());
        let dispatcher = dispatcher.clone();

        // Spawn async task to load PRs
        let mode = if force_refresh { "force refresh" } else { "cached" };
        log::info!(
            "Spawning async task to load PRs for {}/{} ({})",
            org,
            repo_name,
            mode
        );

        self.runtime.spawn(async move {
            log::info!("Async task started: Loading PRs for {}/{}", org, repo_name);

            match client
                .fetch_pull_requests(&org, &repo_name, base_branch.as_deref())
                .await
            {
                Ok(prs) => {
                    let domain_prs: Vec<Pr> = prs.into_iter().map(convert_to_domain_pr).collect();
                    log::info!("Loaded {} PRs for {}/{}", domain_prs.len(), org, repo_name);
                    dispatcher.dispatch(Action::PrLoaded(repo_idx, domain_prs));
                }
                Err(e) => {
                    log::error!("Failed to load PRs for {}/{}: {}", org, repo_name, e);
                    dispatcher.dispatch(Action::PrLoadError(repo_idx, e.to_string()));
                }
            }
        });

        true // Let action pass through to reducer (to set loading state)
    }
}

/// Initialize GitHub client with caching support
async fn init_client(
    cache: Arc<Mutex<ApiCache>>,
) -> anyhow::Result<CachedGitHubClient<OctocrabClient>> {
    // Try environment variables first
    let token = std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .or_else(|_| {
            // Fallback: try to get token from gh CLI
            log::debug!("No GITHUB_TOKEN/GH_TOKEN found, trying gh auth token");
            std::process::Command::new("gh")
                .args(["auth", "token"])
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .ok()
                            .map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                })
                .ok_or(std::env::VarError::NotPresent)
        })
        .map_err(|_| {
            anyhow::anyhow!(
                "GitHub token not found. Set GITHUB_TOKEN, GH_TOKEN, or run 'gh auth login'"
            )
        })?;

    let octocrab = Octocrab::builder().personal_token(token).build()?;
    let octocrab_client = OctocrabClient::new(Arc::new(octocrab));

    // Wrap with caching (ReadWrite mode by default)
    let cached_client = CachedGitHubClient::new(octocrab_client, cache, CacheMode::ReadWrite);

    Ok(cached_client)
}

/// Convert gh-client PullRequest to domain Pr
fn convert_to_domain_pr(pr: PullRequest) -> Pr {
    let mergeable = match pr.mergeable_state {
        Some(gh_client::types::MergeableState::Clean) => MergeableStatus::Ready,
        Some(gh_client::types::MergeableState::Behind) => MergeableStatus::NeedsRebase,
        Some(gh_client::types::MergeableState::Dirty) => MergeableStatus::Conflicted,
        Some(gh_client::types::MergeableState::Blocked) => MergeableStatus::Blocked,
        Some(gh_client::types::MergeableState::Unstable) => MergeableStatus::BuildFailed,
        _ => MergeableStatus::Unknown,
    };

    Pr {
        number: pr.number as usize,
        title: pr.title,
        body: pr.body.unwrap_or_default(),
        author: pr.author,
        comments: pr.comments as usize,
        mergeable,
        needs_rebase: matches!(mergeable, MergeableStatus::NeedsRebase),
        head_sha: pr.head_sha,
        created_at: pr.created_at,
        updated_at: pr.updated_at,
    }
}
