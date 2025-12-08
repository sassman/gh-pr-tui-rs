//! GitHub Operations Middleware
//!
//! Central middleware for all GitHub API interactions:
//! - Client initialization (on BootstrapStart)
//! - PR loading (fetch_pull_requests)
//! - PR operations (merge, rebase, approve, close)
//! - CI operations (rerun failed jobs)
//! - Browser/IDE integration

use crate::actions::{
    Action, BootstrapAction, BuildLogAction, DiffViewerAction, Event, GlobalAction, LoadedComment,
    PullRequestAction, RepositoryAction, StatusBarAction,
};
use crate::dispatcher::Dispatcher;
use crate::domain_models::{MergeableStatus, Pr, Repository};
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::state::{BuildLogJobMetadata, BuildLogJobStatus, BuildLogPrContext};
use crate::utils::browser::open_url;
use crate::views::BuildLogView;
use gh_client::{
    octocrab::Octocrab, ApiCache, CacheMode, CachedGitHubClient, ClientManager, GitHubClient,
    MergeMethod, OctocrabClient, PullRequest, ReviewEvent,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::Mutex as TokioMutex;

/// Middleware for all GitHub API operations
pub struct GitHubMiddleware {
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// Client manager for multi-host support
    client_manager: Arc<TokioMutex<ClientManager>>,
}

impl GitHubMiddleware {
    /// Create a new GitHub middleware
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");

        // Initialize cache from config path
        let cache_file =
            gh_pr_config::api_cache_path().expect("API Cache path should always exist.");
        let cache = Arc::new(Mutex::new(ApiCache::new(cache_file).unwrap_or_default()));

        // Create client manager with shared cache
        let client_manager = ClientManager::new(cache);

        Self {
            runtime,
            client_manager: Arc::new(TokioMutex::new(client_manager)),
        }
    }

    /// Get a client for the default host (github.com) synchronously if available
    /// This is used for quick checks - actual operations should use get_client_for_repo
    fn has_default_client(&self) -> bool {
        // Try to check without blocking
        if let Ok(guard) = self.client_manager.try_lock() {
            guard.has_client(None)
        } else {
            false
        }
    }

    /// Get the client manager Arc for use in async tasks
    fn client_manager_arc(&self) -> Arc<TokioMutex<ClientManager>> {
        Arc::clone(&self.client_manager)
    }

    /// Initialize the GitHub client for the default host (async, non-blocking)
    fn initialize_client(&self, dispatcher: &Dispatcher) {
        let client_manager = self.client_manager_arc();
        let dispatcher = dispatcher.clone();

        self.runtime.spawn(async move {
            let mut manager = client_manager.lock().await;
            match manager.get_client(None).await {
                Ok(_) => {
                    log::info!("GitHubMiddleware: GitHub client initialized for github.com");
                    // Signal that client is ready - trigger any pending operations
                    dispatcher.dispatch(Action::event(Event::ClientReady));
                }
                Err(e) => {
                    log::warn!("GitHubMiddleware: GitHub client not initialized: {}", e);
                }
            }
        });
    }

    /// Get target PRs for an operation (selected PRs or cursor PR)
    /// Returns: Vec<(Repository, pr_number)>
    fn get_target_prs(&self, state: &AppState) -> Vec<(Repository, usize)> {
        let repo_idx = state.main_view.selected_repository;

        let Some(repo) = state.main_view.repositories.get(repo_idx) else {
            return vec![];
        };

        if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
            // If there are selected PRs, use those
            if !repo_data.selected_pr_numbers.is_empty() {
                return repo_data
                    .selected_pr_numbers
                    .iter()
                    .map(|&pr_num| (repo.clone(), pr_num))
                    .collect();
            }

            // Otherwise use the cursor PR
            if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                return vec![(repo.clone(), pr.number)];
            }
        }

        vec![]
    }

    /// Get target PRs with author info for rebase operation
    /// Returns: Vec<(Repository, pr_number, author)>
    fn get_target_prs_with_author(&self, state: &AppState) -> Vec<(Repository, usize, String)> {
        let repo_idx = state.main_view.selected_repository;

        let Some(repo) = state.main_view.repositories.get(repo_idx) else {
            return vec![];
        };

        if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
            // If there are selected PRs, use those
            if !repo_data.selected_pr_numbers.is_empty() {
                return repo_data
                    .prs
                    .iter()
                    .filter(|pr| repo_data.selected_pr_numbers.contains(&pr.number))
                    .map(|pr| (repo.clone(), pr.number, pr.author.clone()))
                    .collect();
            }

            // Otherwise use the cursor PR
            if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                return vec![(repo.clone(), pr.number, pr.author.clone())];
            }
        }

        vec![]
    }

    /// Get target PR URLs for opening in browser (respects multi-selection)
    fn get_target_pr_urls(&self, state: &AppState) -> Vec<String> {
        let repo_idx = state.main_view.selected_repository;

        if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
            // If there are selected PRs, use those
            if !repo_data.selected_pr_numbers.is_empty() {
                return repo_data
                    .prs
                    .iter()
                    .filter(|pr| repo_data.selected_pr_numbers.contains(&pr.number))
                    .map(|pr| pr.html_url.clone())
                    .collect();
            }

            // Otherwise use the cursor PR
            if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                return vec![pr.html_url.clone()];
            }
        }

        vec![]
    }

    /// Get target PR info for IDE opening (respects multi-selection)
    /// Returns: Vec<(pr_number, Repository)>
    fn get_target_pr_info_for_ide(&self, state: &AppState) -> Vec<(usize, Repository)> {
        let repo_idx = state.main_view.selected_repository;

        if let Some(repo) = state.main_view.repositories.get(repo_idx) {
            if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
                // If there are selected PRs, use those
                if !repo_data.selected_pr_numbers.is_empty() {
                    return repo_data
                        .selected_pr_numbers
                        .iter()
                        .map(|&pr_num| (pr_num, repo.clone()))
                        .collect();
                }

                // Otherwise use the cursor PR
                if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                    return vec![(pr.number, repo.clone())];
                }
            }
        }

        vec![]
    }

    /// Get target PR CI info for build operations (respects multi-selection)
    /// Returns: Vec<(Repository, pr_number, head_sha, head_branch)>
    fn get_target_pr_ci_info(&self, state: &AppState) -> Vec<(Repository, u64, String, String)> {
        let repo_idx = state.main_view.selected_repository;

        if let Some(repo) = state.main_view.repositories.get(repo_idx) {
            if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
                // If there are selected PRs, use those
                if !repo_data.selected_pr_numbers.is_empty() {
                    return repo_data
                        .prs
                        .iter()
                        .filter(|pr| repo_data.selected_pr_numbers.contains(&pr.number))
                        .map(|pr| {
                            (
                                repo.clone(),
                                pr.number as u64,
                                pr.head_sha.clone(),
                                pr.head_branch.clone(),
                            )
                        })
                        .collect();
                }

                // Otherwise use the cursor PR
                if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                    return vec![(
                        repo.clone(),
                        pr.number as u64,
                        pr.head_sha.clone(),
                        pr.head_branch.clone(),
                    )];
                }
            }
        }

        vec![]
    }

    /// Get repository context string for confirmation popup
    fn get_repo_context(&self, state: &AppState) -> String {
        let repo_idx = state.main_view.selected_repository;
        state
            .main_view
            .repositories
            .get(repo_idx)
            .map(|r| format!("{}/{}", r.org, r.repo))
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Trigger CI status checks for PRs that don't have status loaded yet
    fn trigger_ci_status_if_needed(
        &self,
        repo_idx: usize,
        state: &AppState,
        dispatcher: &Dispatcher,
    ) {
        if !self.has_default_client() {
            return;
        }

        let Some(repo) = state.main_view.repositories.get(repo_idx) else {
            return;
        };

        // Check if this repository has PRs loaded
        if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
            // Filter PRs with Unknown status
            let prs_needing_status: Vec<_> = repo_data
                .prs
                .iter()
                .filter(|pr| matches!(pr.mergeable, MergeableStatus::Unknown))
                .cloned()
                .collect();

            dispatch_ci_status_checks(repo, &prs_needing_status, dispatcher, self.client_manager_arc());
        }
    }

    fn handle_pr_load_2(
        &self,
        repo: &Repository,
        _state: &AppState,
        dispatcher: &Dispatcher,
        force_refresh: bool,
    ) -> bool {
        log::info!("PrLoad: Loading PRs for {}/{}", repo.org, repo.repo);

        let repo = repo.clone();
        let dispatcher = dispatcher.clone();
        let client_manager = self.client_manager_arc();

        // Spawn async task to load PRs
        let mode = if force_refresh {
            "force refresh"
        } else {
            "cached"
        };
        log::info!(
            "Spawning async task to load PRs for {}/{} ({})",
            repo.org,
            repo.repo,
            mode
        );

        self.runtime.spawn(async move {
            log::info!(
                "Async task started: Loading PRs for {}/{}",
                repo.org,
                repo.repo
            );

            // Get client for this repository's host
            let client = {
                let mut manager = client_manager.lock().await;
                match manager.clone_client(repo.host.as_deref()).await {
                    Ok(c) => if force_refresh { c.with_mode(CacheMode::WriteOnly) } else { c },
                    Err(e) => {
                        log::error!("Failed to get client for host {:?}: {}", repo.host, e);
                        dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                            format!("Failed to connect to GitHub: {}", e),
                            "Load",
                        )));
                        dispatcher.dispatch(Action::PullRequest(PullRequestAction::LoadError {
                            repo,
                            error: e.to_string(),
                        }));
                        return;
                    }
                }
            };

            match client
                .fetch_pull_requests(&repo.org, &repo.repo, Some(&repo.branch))
                .await
            {
                Ok(prs) => {
                    let domain_prs: Vec<Pr> = prs.into_iter().map(convert_to_domain_pr).collect();
                    log::info!(
                        "Loaded {} PRs for {}/{}",
                        domain_prs.len(),
                        repo.org,
                        repo.repo
                    );
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::info(
                        format!(
                            "Loaded {} PRs from {}/{}",
                            domain_prs.len(),
                            repo.org,
                            repo.repo
                        ),
                        "Load",
                    )));

                    // First dispatch Loaded to populate repo_data.prs
                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::Loaded {
                        repo: repo.clone(),
                        prs: domain_prs.clone(),
                    }));

                    // Then trigger CI status checks for each PR (background fetch)
                    // This must come AFTER Loaded so the PRs exist when BuildStatusUpdated arrives
                    dispatch_ci_status_checks(&repo, &domain_prs, &dispatcher, Arc::clone(&client_manager));

                    // Also trigger background fetch for PR stats (additions/deletions)
                    dispatch_pr_stats_fetch(&repo, &domain_prs, &dispatcher, client, Arc::clone(&client_manager));
                }
                Err(e) => {
                    log::error!("Failed to load PRs for {}/{}: {}", repo.org, repo.repo, e);
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                        format!("Failed to load PRs: {}", e),
                        "Load",
                    )));
                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::LoadError {
                        repo,
                        error: e.to_string(),
                    }));
                }
            }
        });

        true // Let action pass through to reducer (to set loading state)
    }

    /// Handle loading PRs for a repository
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

        // Get the repository at this index
        let Some(repo) = state.main_view.repositories.get(repo_idx) else {
            log::warn!(
                "PrLoad: Repository at index {} not found (state has {} repos), will retry",
                repo_idx,
                state.main_view.repositories.len()
            );
            return true;
        };

        self.handle_pr_load_2(repo, state, dispatcher, force_refresh)
    }
}

impl Default for GitHubMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for GitHubMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Initialize client on bootstrap (async, non-blocking)
            Action::Bootstrap(BootstrapAction::Start) => {
                self.initialize_client(dispatcher);
                true // Let action pass through
            }

            // Client ready event - trigger repository loading
            Action::Event(Event::ClientReady) => {
                log::info!("GitHub client ready, triggering repository loading");
                dispatcher.dispatch(Action::Bootstrap(BootstrapAction::LoadRecentRepositories));
                true // Let action pass through
            }

            Action::Repository(RepositoryAction::LoadRepositoryData(repo)) => {
                self.handle_pr_load_2(repo, state, dispatcher, false)
            }

            // Handle PR load start - actually fetch the PRs
            Action::PullRequest(PullRequestAction::LoadStart { repo }) => {
                self.handle_pr_load_2(repo, state, dispatcher, false)
            }

            // Handle PR refresh request (force refresh - bypass cache)
            Action::PullRequest(PullRequestAction::Refresh) => {
                let repo_idx = state.main_view.selected_repository;
                self.handle_pr_load(repo_idx, state, dispatcher, true)
            }

            // Handle repository switching - trigger CI status checks if needed
            Action::PullRequest(PullRequestAction::RepositoryNext) => {
                let num_repos = state.main_view.repositories.len();
                if num_repos > 0 {
                    let next_repo_idx = (state.main_view.selected_repository + 1) % num_repos;
                    self.trigger_ci_status_if_needed(next_repo_idx, state, dispatcher);
                }
                true // Let action pass through to reducer
            }

            Action::PullRequest(PullRequestAction::RepositoryPrevious) => {
                let num_repos = state.main_view.repositories.len();
                if num_repos > 0 {
                    let prev_repo_idx = if state.main_view.selected_repository == 0 {
                        num_repos - 1
                    } else {
                        state.main_view.selected_repository - 1
                    };
                    self.trigger_ci_status_if_needed(prev_repo_idx, state, dispatcher);
                }
                true // Let action pass through to reducer
            }

            Action::PullRequest(PullRequestAction::OpenInBrowser) => {
                let urls = self.get_target_pr_urls(state);
                if urls.is_empty() {
                    log::warn!("No PRs selected for opening in browser");
                    return false;
                }

                log::info!("Opening {} PR(s) in browser", urls.len());

                for url in urls {
                    self.runtime.spawn(open_url(url));
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::MergeRequest) => {
                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for merge");
                    return false;
                }

                let client_manager = self.client_manager_arc();

                for (repo, pr_number) in targets {
                    let dispatcher = dispatcher.clone();
                    let client_manager = Arc::clone(&client_manager);

                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::MergeStart {
                        repo: repo.clone(),
                        pr_number,
                    }));
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                        format!("Merging PR #{}...", pr_number),
                        "Merge",
                    )));

                    self.runtime.spawn(async move {
                        // Get client for this repository's host
                        let client = {
                            let mut manager = client_manager.lock().await;
                            match manager.clone_client(repo.host.as_deref()).await {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("Failed to get client: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Merge error: {}", e),
                                        "Merge",
                                    )));
                                    return;
                                }
                            }
                        };

                        match client
                            .merge_pull_request(
                                &repo.org,
                                &repo.repo,
                                pr_number as u64,
                                MergeMethod::default(),
                                None,
                                None,
                            )
                            .await
                        {
                            Ok(result) if result.merged => {
                                log::info!("Successfully merged PR #{}", pr_number);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                    format!("PR #{} merged", pr_number),
                                    "Merge",
                                )));
                                // Trigger refresh to update PR list
                                dispatcher
                                    .dispatch(Action::PullRequest(PullRequestAction::Refresh));
                            }
                            Ok(result) => {
                                log::error!("Merge failed: {}", result.message);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Merge failed: {}", result.message),
                                    "Merge",
                                )));
                            }
                            Err(e) => {
                                log::error!("Merge error: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Merge error: {}", e),
                                    "Merge",
                                )));
                            }
                        }
                    });
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::RebaseRequest) => {
                let targets = self.get_target_prs_with_author(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for rebase");
                    return false;
                }

                let client_manager = self.client_manager_arc();

                for (repo, pr_number, author) in targets {
                    let is_dependabot = author.to_lowercase().contains("dependabot");
                    let dispatcher = dispatcher.clone();
                    let client_manager = Arc::clone(&client_manager);

                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::RebaseStart {
                        repo: repo.clone(),
                        pr_number,
                    }));
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                        format!("Updating branch for PR #{}...", pr_number),
                        "Rebase",
                    )));

                    self.runtime.spawn(async move {
                        // Get client for this repository's host
                        let client = {
                            let mut manager = client_manager.lock().await;
                            match manager.clone_client(repo.host.as_deref()).await {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("Failed to get client: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Rebase failed: {}", e),
                                        "Rebase",
                                    )));
                                    return;
                                }
                            }
                        };

                        if is_dependabot {
                            // For dependabot PRs, post a comment to trigger rebase
                            let octocrab = client.inner().octocrab_arc();
                            match octocrab
                                .issues(&repo.org, &repo.repo)
                                .create_comment(pr_number as u64, "@dependabot rebase")
                                .await
                            {
                                Ok(_) => {
                                    log::info!("Requested dependabot rebase for PR #{}", pr_number);
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!(
                                                "Dependabot rebase requested for PR #{}",
                                                pr_number
                                            ),
                                            "Rebase",
                                        ),
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Dependabot rebase request error: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Rebase request failed: {}", e),
                                        "Rebase",
                                    )));
                                }
                            }
                        } else {
                            // For regular PRs, use the update branch API
                            match client
                                .update_pull_request_branch(&repo.org, &repo.repo, pr_number as u64)
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully rebased PR #{}", pr_number);
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!("PR #{} branch updated", pr_number),
                                            "Rebase",
                                        ),
                                    ));
                                    // Trigger refresh to update PR status
                                    dispatcher
                                        .dispatch(Action::PullRequest(PullRequestAction::Refresh));
                                }
                                Err(e) => {
                                    log::error!("Rebase error: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Rebase failed: {}", e),
                                        "Rebase",
                                    )));
                                }
                            }
                        }
                    });
                }
                false // Consume action
            }

            // PR Actions that show confirmation popup
            Action::PullRequest(PullRequestAction::ApproveRequest) => {
                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for approval");
                    return false;
                }

                let pr_numbers: Vec<u64> = targets.iter().map(|(_, pr)| *pr as u64).collect();
                let repo_context = self.get_repo_context(state);
                let default_message = state.app_config.approval_message.clone();

                dispatcher.dispatch(Action::ConfirmationPopup(
                    crate::actions::ConfirmationPopupAction::Show {
                        intent: crate::state::ConfirmationIntent::Approve { pr_numbers },
                        default_message,
                        repo_context,
                    },
                ));
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::CommentRequest) => {
                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for comment");
                    return false;
                }

                let pr_numbers: Vec<u64> = targets.iter().map(|(_, pr)| *pr as u64).collect();
                let repo_context = self.get_repo_context(state);
                let default_message = state.app_config.comment_message.clone();

                dispatcher.dispatch(Action::ConfirmationPopup(
                    crate::actions::ConfirmationPopupAction::Show {
                        intent: crate::state::ConfirmationIntent::Comment { pr_numbers },
                        default_message,
                        repo_context,
                    },
                ));
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::RequestChangesRequest) => {
                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for request changes");
                    return false;
                }

                let pr_numbers: Vec<u64> = targets.iter().map(|(_, pr)| *pr as u64).collect();
                let repo_context = self.get_repo_context(state);
                let default_message = state.app_config.request_changes_message.clone();

                dispatcher.dispatch(Action::ConfirmationPopup(
                    crate::actions::ConfirmationPopupAction::Show {
                        intent: crate::state::ConfirmationIntent::RequestChanges { pr_numbers },
                        default_message,
                        repo_context,
                    },
                ));
                false // Consume action
            }

            // Actual execution actions (from confirmation popup)
            Action::PullRequest(PullRequestAction::ApproveWithMessage {
                pr_numbers,
                message,
            }) => {
                let repo_idx = state.main_view.selected_repository;
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    log::error!("No repository selected");
                    return false;
                };

                let message = if message.is_empty() {
                    None
                } else {
                    Some(message.clone())
                };

                let client_manager = self.client_manager_arc();

                for pr_number in pr_numbers {
                    let dispatcher = dispatcher.clone();
                    let client_manager = Arc::clone(&client_manager);
                    let message = message.clone();
                    let pr_num = *pr_number as usize;
                    let pr_number_owned = *pr_number;
                    let repo = repo.clone();

                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::ApproveStart {
                        repo: repo.clone(),
                        pr_number: pr_num,
                    }));
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                        format!("Approving PR #{}...", pr_number_owned),
                        "Approve",
                    )));

                    self.runtime.spawn(async move {
                        let client = {
                            let mut manager = client_manager.lock().await;
                            match manager.clone_client(repo.host.as_deref()).await {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("Failed to get client: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Approve failed: {}", e),
                                        "Approve",
                                    )));
                                    return;
                                }
                            }
                        };

                        match client
                            .create_review(
                                &repo.org,
                                &repo.repo,
                                pr_number_owned,
                                ReviewEvent::Approve,
                                message.as_deref(),
                            )
                            .await
                        {
                            Ok(()) => {
                                log::info!("Successfully approved PR #{}", pr_number_owned);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                    format!("PR #{} approved", pr_number_owned),
                                    "Approve",
                                )));
                            }
                            Err(e) => {
                                log::error!("Approve error: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Approve failed: {}", e),
                                    "Approve",
                                )));
                            }
                        }
                    });
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::CommentOnPr {
                pr_numbers,
                message,
            }) => {
                let repo_idx = state.main_view.selected_repository;
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    log::error!("No repository selected");
                    return false;
                };

                let client_manager = self.client_manager_arc();

                for pr_number in pr_numbers {
                    let dispatcher = dispatcher.clone();
                    let client_manager = Arc::clone(&client_manager);
                    let message = message.clone();
                    let pr_num = *pr_number as usize;
                    let pr_number_owned = *pr_number;
                    let repo = repo.clone();

                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::CommentStart {
                        repo: repo.clone(),
                        pr_number: pr_num,
                    }));
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                        format!("Commenting on PR #{}...", pr_number_owned),
                        "Comment",
                    )));

                    self.runtime.spawn(async move {
                        let client = {
                            let mut manager = client_manager.lock().await;
                            match manager.clone_client(repo.host.as_deref()).await {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("Failed to get client: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Comment failed: {}", e),
                                        "Comment",
                                    )));
                                    return;
                                }
                            }
                        };

                        let octocrab = client.inner().octocrab_arc();
                        match octocrab
                            .issues(&repo.org, &repo.repo)
                            .create_comment(pr_number_owned, &message)
                            .await
                        {
                            Ok(_) => {
                                log::info!("Successfully commented on PR #{}", pr_number_owned);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                    format!("Commented on PR #{}", pr_number_owned),
                                    "Comment",
                                )));
                            }
                            Err(e) => {
                                log::error!("Comment error: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Comment failed: {}", e),
                                    "Comment",
                                )));
                            }
                        }
                    });
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::RequestChanges {
                pr_numbers,
                message,
            }) => {
                let repo_idx = state.main_view.selected_repository;
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    log::error!("No repository selected");
                    return false;
                };

                let client_manager = self.client_manager_arc();

                for pr_number in pr_numbers {
                    let dispatcher = dispatcher.clone();
                    let client_manager = Arc::clone(&client_manager);
                    let message = message.clone();
                    let pr_num = *pr_number as usize;
                    let pr_number_owned = *pr_number;
                    let repo = repo.clone();

                    dispatcher.dispatch(Action::PullRequest(
                        PullRequestAction::RequestChangesStart {
                            repo: repo.clone(),
                            pr_number: pr_num,
                        },
                    ));
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                        format!("Requesting changes on PR #{}...", pr_number_owned),
                        "Request Changes",
                    )));

                    self.runtime.spawn(async move {
                        let client = {
                            let mut manager = client_manager.lock().await;
                            match manager.clone_client(repo.host.as_deref()).await {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("Failed to get client: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Request changes failed: {}", e),
                                        "Request Changes",
                                    )));
                                    return;
                                }
                            }
                        };

                        match client
                            .create_review(
                                &repo.org,
                                &repo.repo,
                                pr_number_owned,
                                ReviewEvent::RequestChanges,
                                Some(&message),
                            )
                            .await
                        {
                            Ok(()) => {
                                log::info!(
                                    "Successfully requested changes on PR #{}",
                                    pr_number_owned
                                );
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                    format!("Requested changes on PR #{}", pr_number_owned),
                                    "Request Changes",
                                )));
                            }
                            Err(e) => {
                                log::error!("Request changes error: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Request changes failed: {}", e),
                                    "Request Changes",
                                )));
                            }
                        }
                    });
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::CloseRequest) => {
                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for closing");
                    return false;
                }

                let pr_numbers: Vec<u64> = targets.iter().map(|(_, pr)| *pr as u64).collect();
                let repo_context = self.get_repo_context(state);
                let default_message = state.app_config.close_message.clone();

                dispatcher.dispatch(Action::ConfirmationPopup(
                    crate::actions::ConfirmationPopupAction::Show {
                        intent: crate::state::ConfirmationIntent::Close { pr_numbers },
                        default_message,
                        repo_context,
                    },
                ));
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::ClosePrWithMessage {
                pr_numbers,
                message,
            }) => {
                let repo_idx = state.main_view.selected_repository;
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    log::error!("No repository selected");
                    return false;
                };

                let client_manager = Arc::clone(&self.client_manager);

                for pr_number in pr_numbers {
                    let dispatcher = dispatcher.clone();
                    let message = message.clone();
                    let pr_num = *pr_number as usize;
                    let pr_number_owned = *pr_number;
                    let repo = repo.clone();
                    let client_manager = Arc::clone(&client_manager);

                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::CloseStart {
                        repo: repo.clone(),
                        pr_number: pr_num,
                    }));
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                        format!("Closing PR #{}...", pr_number_owned),
                        "Close",
                    )));

                    self.runtime.spawn(async move {
                        // Get client inside async task
                        let client = {
                            let mut manager = client_manager.lock().await;
                            match manager.clone_client(repo.host.as_deref()).await {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("Failed to get client: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Close failed: {}", e),
                                        "Close",
                                    )));
                                    return;
                                }
                            }
                        };

                        // Post comment if message is not empty
                        if !message.is_empty() {
                            if let Err(e) = client
                                .inner()
                                .octocrab_arc()
                                .issues(&repo.org, &repo.repo)
                                .create_comment(pr_number_owned, &message)
                                .await
                            {
                                log::warn!(
                                    "Failed to post close comment on PR #{}: {}",
                                    pr_number_owned,
                                    e
                                );
                            }
                        }

                        // Close the PR
                        match client
                            .close_pull_request(&repo.org, &repo.repo, pr_number_owned)
                            .await
                        {
                            Ok(()) => {
                                log::info!("Successfully closed PR #{}", pr_number_owned);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                    format!("PR #{} closed", pr_number_owned),
                                    "Close",
                                )));
                                // Trigger refresh to update PR list
                                dispatcher
                                    .dispatch(Action::PullRequest(PullRequestAction::Refresh));
                            }
                            Err(e) => {
                                log::error!("Close error: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Close failed: {}", e),
                                    "Close",
                                )));
                            }
                        }
                    });
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::OpenBuildLogs) => {
                let targets = self.get_target_pr_ci_info(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for opening build logs");
                    return false;
                }

                log::info!("Opening build logs for {} PR(s)", targets.len());

                for (repo, _pr_number, _head_sha, head_branch) in targets {
                    let url = format!(
                        "{}/actions?query=branch%3A{}",
                        repo.web_url(), head_branch
                    );
                    self.runtime.spawn(open_url(url));
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::OpenInIDE) => {
                let targets = self.get_target_pr_info_for_ide(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for opening in IDE");
                    return false;
                }

                log::info!("Opening {} PR(s) in IDE", targets.len());

                // Get config values before spawning (they need to be moved into the closure)
                let ide_command = state.app_config.ide_command.clone();
                let temp_dir_base = state.app_config.temp_dir.clone();

                // Spawn blocking task for each PR to open in IDE
                for (pr_number, repo) in targets {
                    let ide_command = ide_command.clone();
                    let temp_dir_base = temp_dir_base.clone();
                    let org = repo.org.clone();
                    let repo_name = repo.repo.clone();
                    let repo_host = repo.host.clone();
                    let ssh_url = repo.ssh_url();

                    self.runtime.spawn_blocking(move || {
                        use std::path::PathBuf;
                        use std::process::Command;

                        let temp_dir = PathBuf::from(&temp_dir_base);

                        // Create temp directory if it doesn't exist
                        if let Err(err) = std::fs::create_dir_all(&temp_dir) {
                            log::error!("Failed to create temp directory: {}", err);
                            return;
                        }

                        // Create unique directory for this PR (include host for GHE)
                        let host_prefix = match &repo_host {
                            Some(h) if h != gh_client::DEFAULT_HOST => {
                                // Sanitize hostname for filesystem (replace dots with dashes)
                                format!("{}-", h.replace('.', "-"))
                            }
                            _ => String::new(),
                        };
                        let dir_name = format!("{}{}-{}-pr-{}", host_prefix, org, repo_name, pr_number);
                        let pr_dir = temp_dir.join(dir_name);

                        // Remove existing directory if present
                        if pr_dir.exists() {
                            if let Err(err) = std::fs::remove_dir_all(&pr_dir) {
                                log::error!("Failed to remove existing directory: {}", err);
                                return;
                            }
                        }

                        // Clone the repository using gh repo clone
                        log::info!("Cloning {}/{} to {:?}", org, repo_name, pr_dir);
                        let mut clone_args = vec![
                            "repo".to_string(),
                            "clone".to_string(),
                            format!("{}/{}", org, repo_name),
                            pr_dir.to_string_lossy().to_string(),
                        ];
                        // Add --hostname for GitHub Enterprise hosts
                        if let Some(ref host) = repo_host {
                            if host != gh_client::DEFAULT_HOST {
                                clone_args.push("--hostname".to_string());
                                clone_args.push(host.clone());
                            }
                        }
                        let clone_output = Command::new("gh")
                            .args(&clone_args)
                            .output();

                        match clone_output {
                            Err(err) => {
                                log::error!("Failed to run gh repo clone: {}", err);
                                return;
                            }
                            Ok(output) if !output.status.success() => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                log::error!("gh repo clone failed: {}", stderr);
                                return;
                            }
                            _ => {}
                        }

                        // Checkout the PR using gh pr checkout
                        log::info!("Checking out PR #{}", pr_number);
                        let checkout_output = Command::new("gh")
                            .args(["pr", "checkout", &pr_number.to_string()])
                            .current_dir(&pr_dir)
                            .output();

                        match checkout_output {
                            Err(err) => {
                                log::error!("Failed to run gh pr checkout: {}", err);
                                return;
                            }
                            Ok(output) if !output.status.success() => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                log::error!("gh pr checkout failed: {}", stderr);
                                return;
                            }
                            _ => {}
                        }

                        // Set origin URL to SSH (gh checkout doesn't do this)
                        let set_url_output = Command::new("git")
                            .args(["remote", "set-url", "origin", &ssh_url])
                            .current_dir(&pr_dir)
                            .output();

                        if let Err(err) = set_url_output {
                            log::warn!("Failed to set SSH origin URL: {}", err);
                            // Continue anyway - HTTPS will still work
                        }

                        // Open in configured IDE
                        if Command::new(&ide_command).arg(&pr_dir).spawn().is_ok() {
                            log::info!(
                                "Opened PR #{} in {} at {:?}",
                                pr_number,
                                ide_command,
                                pr_dir
                            );
                        } else {
                            log::error!(
                                "Failed to open IDE '{}'. PR cloned at: {:?}",
                                ide_command,
                                pr_dir
                            );
                        }
                    });
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::RerunFailedJobs) => {
                let targets = self.get_target_pr_ci_info(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for rerunning jobs");
                    return false;
                }

                log::info!("Rerunning failed jobs for {} PR(s)", targets.len());

                let client_manager = Arc::clone(&self.client_manager);

                // Rerun failed jobs for each target PR
                for (repo, pr_number, head_sha, _head_branch) in targets {
                    let dispatcher = dispatcher.clone();
                    let client_manager = Arc::clone(&client_manager);

                    // Fetch workflow runs, then rerun failed ones
                    self.runtime.spawn(async move {
                        // Get client inside async task
                        let client = {
                            let mut manager = client_manager.lock().await;
                            match manager.clone_client(repo.host.as_deref()).await {
                                Ok(c) => c,
                                Err(e) => {
                                    log::error!("Failed to get client for rerun: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Rerun failed: {}", e),
                                        "Rerun",
                                    )));
                                    return;
                                }
                            }
                        };

                        // Fetch workflow runs for this commit
                        match client.fetch_workflow_runs(&repo.org, &repo.repo, &head_sha).await {
                            Ok(runs) => {
                                // Filter to failed runs and rerun each
                                let failed_runs: Vec<_> = runs
                                    .into_iter()
                                    .filter(|r| {
                                        r.conclusion.as_ref().is_some_and(|c| {
                                            matches!(
                                                c,
                                                gh_client::WorkflowRunConclusion::Failure
                                                    | gh_client::WorkflowRunConclusion::TimedOut
                                            )
                                        })
                                    })
                                    .collect();

                                if failed_runs.is_empty() {
                                    log::info!(
                                        "No failed workflow runs to rerun for PR #{}",
                                        pr_number
                                    );
                                    return;
                                }

                                for run in failed_runs {
                                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::RerunStart {
                                        repo: repo.clone(),
                                        pr_number,
                                        run_id: run.id,
                                    }));

                                    match client.rerun_failed_jobs(&repo.org, &repo.repo, run.id).await {
                                        Ok(()) => {
                                            log::info!(
                                                "Successfully triggered rerun for workflow {} (PR #{})",
                                                run.name,
                                                pr_number
                                            );
                                            dispatcher.dispatch(Action::StatusBar(
                                                StatusBarAction::success(
                                                    format!("Rerun triggered for {} (PR #{})", run.name, pr_number),
                                                    "Rerun",
                                                ),
                                            ));
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "Failed to rerun workflow {} (PR #{}): {}",
                                                run.name,
                                                pr_number,
                                                e
                                            );
                                            dispatcher.dispatch(Action::StatusBar(
                                                StatusBarAction::error(
                                                    format!("Rerun failed: {}", e),
                                                    "Rerun",
                                                ),
                                            ));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to fetch workflow runs for PR #{}: {}", pr_number, e);
                            }
                        }
                    });
                }
                false // Consume action
            }

            // === Build Log Operations ===
            Action::BuildLog(BuildLogAction::Open) => {
                let repo_idx = state.main_view.selected_repository;

                // Get repository info
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        "No repository selected",
                        "Build Logs",
                    )));
                    return false;
                };

                // Get repository data
                let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        "No repository data loaded",
                        "Build Logs",
                    )));
                    return false;
                };

                // Get current PR
                let Some(pr) = repo_data.prs.get(repo_data.selected_pr) else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        "No PR selected",
                        "Build Logs",
                    )));
                    return false;
                };

                // Capture PR context
                let pr_context = BuildLogPrContext {
                    number: pr.number,
                    title: pr.title.clone(),
                    author: pr.author.clone(),
                };
                let pr_number = pr.number;
                let head_sha = pr.head_sha.clone();
                let repo_org = repo.org.clone();
                let repo_name = repo.repo.clone();
                let dispatcher = dispatcher.clone();
                let client_manager = Arc::clone(&self.client_manager);

                // Dispatch loading state and push view
                dispatcher.dispatch(Action::BuildLog(BuildLogAction::LoadStart));
                dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                    format!("Loading build logs for PR #{}...", pr_number),
                    "Build Logs",
                )));
                dispatcher.dispatch(Action::Global(GlobalAction::PushView(Box::new(
                    BuildLogView::new(),
                ))));

                // Spawn async task to fetch build logs
                self.runtime.spawn(async move {
                    // Get octocrab client inside async task
                    let octocrab = {
                        let mut manager = client_manager.lock().await;
                        match manager.clone_client(repo.host.as_deref()).await {
                            Ok(c) => c.inner().octocrab_arc(),
                            Err(e) => {
                                log::error!("Failed to get client for build logs: {}", e);
                                dispatcher.dispatch(Action::BuildLog(BuildLogAction::LoadError(
                                    e.to_string(),
                                )));
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Failed to load build logs: {}", e),
                                    "Build Logs",
                                )));
                                return;
                            }
                        }
                    };

                    match fetch_build_logs(
                        &octocrab,
                        &repo_org,
                        &repo_name,
                        &head_sha,
                        pr_context.clone(),
                    )
                    .await
                    {
                        Ok((workflows, job_metadata)) => {
                            dispatcher.dispatch(Action::BuildLog(BuildLogAction::Loaded {
                                workflows,
                                job_metadata,
                                pr_context: pr_context.clone(),
                            }));
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                format!("Build logs loaded for PR #{}", pr_number),
                                "Build Logs",
                            )));
                        }
                        Err(e) => {
                            log::error!("Failed to load build logs: {}", e);
                            dispatcher
                                .dispatch(Action::BuildLog(BuildLogAction::LoadError(e.clone())));
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                format!("Failed to load build logs: {}", e),
                                "Build Logs",
                            )));
                        }
                    }
                });

                false // Consume action
            }

            // Handle CI status check request
            Action::PullRequest(PullRequestAction::CheckBuildStatus {
                repo,
                pr_number,
                head_sha,
            }) => {
                let repo = repo.clone();
                let pr_number = *pr_number;
                let head_sha = head_sha.clone();
                let dispatcher = dispatcher.clone();
                let client_manager = self.client_manager_arc();

                // Spawn async task to fetch CI status
                log::info!("Spawning CI status fetch for PR #{}", pr_number);
                self.runtime.spawn(async move {
                    // Get client for this repository's host
                    let client = {
                        let mut manager = client_manager.lock().await;
                        match manager.clone_client(repo.host.as_deref()).await {
                            Ok(c) => c,
                            Err(e) => {
                                log::warn!("Cannot check build status: {}", e);
                                return;
                            }
                        }
                    };

                    match client.fetch_ci_status(&repo.org, &repo.repo, &head_sha).await {
                        Ok(ci_status) => {
                            // Convert CiState to MergeableStatus using From trait
                            let status: MergeableStatus = ci_status.state.into();
                            log::info!(
                                "CI status fetched for PR #{}: {:?} (passed: {}, failed: {}, pending: {})",
                                pr_number,
                                status,
                                ci_status.passed,
                                ci_status.failed,
                                ci_status.pending
                            );
                            dispatcher.dispatch(Action::PullRequest(
                                PullRequestAction::BuildStatusUpdated {
                                    repo,
                                    pr_number,
                                    status,
                                },
                            ));
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to fetch CI status for PR #{}: {}",
                                pr_number,
                                e
                            );
                            // Don't dispatch error - just leave status as-is
                        }
                    }
                });

                true // Let action pass through
            }

            // === Diff Viewer Operations ===
            Action::DiffViewer(DiffViewerAction::SubmitReviewRequest { pr_number, event }) => {
                let repo_idx = state.main_view.selected_repository;
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    log::error!("No repository selected for review submission");
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                        "No repository selected",
                        "Review",
                    )));
                    return false;
                };

                let pr_number = *pr_number;
                let event = *event;
                let dispatcher = dispatcher.clone();
                let client_manager = self.client_manager_arc();

                // Convert gh_diff_viewer::ReviewEvent to gh_client::ReviewEvent
                let api_event = match event {
                    gh_diff_viewer::ReviewEvent::Approve => ReviewEvent::Approve,
                    gh_diff_viewer::ReviewEvent::RequestChanges => ReviewEvent::RequestChanges,
                    gh_diff_viewer::ReviewEvent::Comment => ReviewEvent::Comment,
                };

                let event_name = match event {
                    gh_diff_viewer::ReviewEvent::Approve => "Approve",
                    gh_diff_viewer::ReviewEvent::RequestChanges => "Request Changes",
                    gh_diff_viewer::ReviewEvent::Comment => "Comment",
                };

                dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                    format!("Submitting {} review for PR #{}...", event_name, pr_number),
                    "Review",
                )));

                self.runtime.spawn(async move {
                    let client = {
                        let mut manager = client_manager.lock().await;
                        match manager.clone_client(repo.host.as_deref()).await {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Failed to get client: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Review failed: {}", e),
                                    "Review",
                                )));
                                return;
                            }
                        }
                    };

                    match client
                        .create_review(&repo.org, &repo.repo, pr_number, api_event, None)
                        .await
                    {
                        Ok(()) => {
                            log::info!(
                                "Successfully submitted {} review for PR #{}",
                                event_name,
                                pr_number
                            );
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                format!("{} review submitted for PR #{}", event_name, pr_number),
                                "Review",
                            )));
                        }
                        Err(e) => {
                            log::error!("Review submission error: {}", e);
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                format!("Review failed: {}", e),
                                "Review",
                            )));
                        }
                    }
                });

                false // Consume action
            }

            Action::DiffViewer(DiffViewerAction::SubmitCommentRequest {
                pr_number,
                head_sha,
                path,
                line,
                side,
                body,
            }) => {
                let repo_idx = state.main_view.selected_repository;
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    log::error!("No repository selected for comment submission");
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                        "No repository selected",
                        "Comment",
                    )));
                    return false;
                };

                let pr_number = *pr_number;
                let head_sha = head_sha.clone();
                let path = path.clone();
                let line = *line;
                let side = side.clone();
                let body = body.clone();
                let dispatcher = dispatcher.clone();
                let client_manager = self.client_manager_arc();

                dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                    format!("Posting comment on PR #{}...", pr_number),
                    "Comment",
                )));

                let path_clone = path.clone();
                let side_clone = side.clone();
                self.runtime.spawn(async move {
                    let client = {
                        let mut manager = client_manager.lock().await;
                        match manager.clone_client(repo.host.as_deref()).await {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Failed to get client: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Comment failed: {}", e),
                                    "Comment",
                                )));
                                return;
                            }
                        }
                    };

                    match client
                        .create_review_comment(
                            &repo.org, &repo.repo, pr_number, &head_sha, &path, line, &side, &body,
                        )
                        .await
                    {
                        Ok(github_id) => {
                            log::info!(
                                "Successfully posted comment on PR #{} at {}:{} (id: {})",
                                pr_number,
                                path,
                                line,
                                github_id
                            );
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                format!("Comment posted on {}:{}", path, line),
                                "Comment",
                            )));
                            // Update local state with the GitHub comment ID
                            dispatcher.dispatch(Action::DiffViewer(
                                DiffViewerAction::CommentPosted {
                                    path: path_clone,
                                    line,
                                    side: side_clone,
                                    github_id,
                                },
                            ));
                        }
                        Err(e) => {
                            log::error!("Comment submission error: {}", e);
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                format!("Comment failed: {}", e),
                                "Comment",
                            )));
                        }
                    }
                });

                false // Consume action
            }

            Action::DiffViewer(DiffViewerAction::DeleteCommentRequest {
                pr_number,
                github_id,
                path,
                line,
                side,
            }) => {
                let repo_idx = state.main_view.selected_repository;
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    log::error!("No repository selected for comment deletion");
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                        "No repository selected",
                        "Comment",
                    )));
                    return false;
                };

                let pr_number = *pr_number;
                let github_id = *github_id;
                let path = path.clone();
                let line = *line;
                let side = side.clone();
                let dispatcher = dispatcher.clone();
                let client_manager = self.client_manager_arc();

                log::info!(
                    "Deleting comment {} on PR #{} at {}:{}",
                    github_id,
                    pr_number,
                    path,
                    line
                );

                dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                    format!("Deleting comment on {}:{}...", path, line),
                    "Comment",
                )));

                let path_clone = path.clone();
                let side_clone = side.clone();
                self.runtime.spawn(async move {
                    let client = {
                        let mut manager = client_manager.lock().await;
                        match manager.clone_client(repo.host.as_deref()).await {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Failed to get client: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Delete failed: {}", e),
                                    "Comment",
                                )));
                                return;
                            }
                        }
                    };

                    match client
                        .delete_review_comment(&repo.org, &repo.repo, github_id)
                        .await
                    {
                        Ok(()) => {
                            log::info!(
                                "Successfully deleted comment {} on PR #{} at {}:{}",
                                github_id,
                                pr_number,
                                path,
                                line
                            );
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::success(
                                format!("Comment deleted from {}:{}", path, line),
                                "Comment",
                            )));
                            // Update local state to remove the comment
                            dispatcher.dispatch(Action::DiffViewer(
                                DiffViewerAction::CommentDeleted {
                                    path: path_clone,
                                    line,
                                    side: side_clone,
                                },
                            ));
                        }
                        Err(e) => {
                            log::error!("Comment deletion error: {}", e);
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                format!("Delete failed: {}", e),
                                "Comment",
                            )));
                        }
                    }
                });

                false // Consume action
            }

            Action::DiffViewer(DiffViewerAction::Open) => {
                let repo_idx = state.main_view.selected_repository;

                // Get repository info
                let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        "No repository selected",
                        "Diff Viewer",
                    )));
                    return false;
                };

                // Get repository data
                let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        "No repository data loaded",
                        "Diff Viewer",
                    )));
                    return false;
                };

                // Get current PR
                let Some(pr) = repo_data.prs.get(repo_data.selected_pr) else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        "No PR selected",
                        "Diff Viewer",
                    )));
                    return false;
                };

                // Capture PR context
                let pr_number = pr.number as u64;
                let pr_title = pr.title.clone();
                let head_sha = pr.head_sha.clone();
                let base_sha = String::new(); // We'll get this from the API
                let repo_org = repo.org.clone();
                let repo_name = repo.repo.clone();
                let repo_host = repo.host.clone();
                let dispatcher = dispatcher.clone();
                let client_manager = self.client_manager_arc();

                // Dispatch loading state
                dispatcher.dispatch(Action::DiffViewer(DiffViewerAction::LoadStart));
                dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                    format!("Loading diff for PR #{}...", pr_number),
                    "Diff Viewer",
                )));

                // Spawn async task to fetch diff and comments
                self.runtime.spawn(async move {
                    // Get client for this repository's host
                    let client = {
                        let mut manager = client_manager.lock().await;
                        match manager.clone_client(repo.host.as_deref()).await {
                            Ok(c) => c,
                            Err(e) => {
                                log::error!("Failed to get client: {}", e);
                                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                    format!("Failed to load diff: {}", e),
                                    "Diff Viewer",
                                )));
                                dispatcher.dispatch(Action::DiffViewer(DiffViewerAction::LoadError(
                                    e.to_string(),
                                )));
                                return;
                            }
                        }
                    };

                    let octocrab = client.inner().octocrab_arc();

                    // Fetch diff
                    let diff_result: Result<String, String> =
                        fetch_pr_diff(&octocrab, &repo_org, &repo_name, pr_number, repo_host.as_deref()).await;

                    // Fetch comments (non-blocking failure)
                    let api_comments: Vec<gh_client::ReviewComment> = client
                        .fetch_review_comments(&repo_org, &repo_name, pr_number)
                        .await
                        .unwrap_or_else(|e| {
                            log::warn!("Failed to fetch review comments: {}", e);
                            vec![]
                        });

                    match diff_result {
                        Ok(diff_text) => {
                            // Parse the diff
                            match gh_diff_viewer::parse_unified_diff(
                                &diff_text, &base_sha, &head_sha,
                            ) {
                                Ok(diff) => {
                                    // Convert API comments to LoadedComment
                                    let comments: Vec<LoadedComment> = api_comments
                                        .into_iter()
                                        .map(|c| LoadedComment {
                                            github_id: c.id,
                                            path: c.path,
                                            line: c.line,
                                            side: c.side,
                                            body: c.body,
                                        })
                                        .collect();

                                    dispatcher.dispatch(Action::DiffViewer(
                                        DiffViewerAction::Loaded {
                                            diff,
                                            pr_number,
                                            pr_title,
                                            head_sha: head_sha.clone(),
                                            comments,
                                        },
                                    ));
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!("Diff loaded for PR #{}", pr_number),
                                            "Diff Viewer",
                                        ),
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Failed to parse diff: {}", e);
                                    dispatcher.dispatch(Action::DiffViewer(
                                        DiffViewerAction::LoadError(format!(
                                            "Failed to parse diff: {}",
                                            e
                                        )),
                                    ));
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Failed to parse diff: {}", e),
                                        "Diff Viewer",
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to fetch diff: {}", e);
                            dispatcher.dispatch(Action::DiffViewer(DiffViewerAction::LoadError(
                                e.clone(),
                            )));
                            dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                format!("Failed to fetch diff: {}", e),
                                "Diff Viewer",
                            )));
                        }
                    }
                });

                true // Let action pass through to push view
            }

            _ => true, // Pass through other actions
        }
    }
}

/// Fetch build logs from GitHub Actions
async fn fetch_build_logs(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    head_sha: &str,
    _pr_context: BuildLogPrContext,
) -> Result<
    (
        Vec<gh_actions_log_parser::WorkflowNode>,
        Vec<BuildLogJobMetadata>,
    ),
    String,
> {
    // Get workflow runs for this commit
    let url = format!(
        "/repos/{}/{}/actions/runs?head_sha={}",
        owner, repo, head_sha
    );

    #[derive(Debug, serde::Deserialize)]
    struct WorkflowRunsResponse {
        workflow_runs: Vec<WorkflowRunData>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct WorkflowRunData {
        id: u64,
        name: String,
        #[allow(dead_code)]
        conclusion: Option<String>,
    }

    let workflow_runs: WorkflowRunsResponse = octocrab
        .get(&url, None::<&()>)
        .await
        .map_err(|e| format!("Failed to fetch workflow runs: {}", e))?;

    let mut all_workflows = Vec::new();
    let mut all_job_metadata = Vec::new();

    // Process each workflow run
    for workflow_run in workflow_runs.workflow_runs {
        let workflow_name = workflow_run.name.clone();

        // Fetch jobs for this workflow run
        let jobs_url = format!(
            "/repos/{}/{}/actions/runs/{}/jobs",
            owner, repo, workflow_run.id
        );

        #[derive(Debug, serde::Deserialize)]
        struct JobsResponse {
            jobs: Vec<WorkflowJob>,
        }

        #[derive(Debug, serde::Deserialize)]
        struct WorkflowJob {
            #[allow(dead_code)]
            id: u64,
            name: String,
            html_url: String,
            conclusion: Option<String>,
            started_at: Option<String>,
            completed_at: Option<String>,
        }

        let jobs_response: Result<JobsResponse, _> = octocrab.get(&jobs_url, None::<&()>).await;

        // Try to download and parse workflow logs
        // Convert u64 to RunId using .into()
        match octocrab
            .actions()
            .download_workflow_run_logs(owner, repo, workflow_run.id.into())
            .await
        {
            Ok(log_data) => {
                // Parse the zip file using gh-actions-log-parser
                match gh_actions_log_parser::parse_workflow_logs(&log_data) {
                    Ok(parsed_log) => {
                        // Build workflow node from parsed log
                        let mut workflow_node = gh_actions_log_parser::WorkflowNode {
                            name: workflow_name.clone(),
                            jobs: Vec::new(),
                            has_failures: false,
                            total_errors: 0,
                        };

                        // Process each job from the parsed log
                        for job_log in parsed_log.jobs {
                            // Find matching GitHub API job by name
                            let github_job = if let Ok(ref jobs) = jobs_response {
                                jobs.jobs.iter().find(|j| job_log.name.contains(&j.name))
                            } else {
                                None
                            };

                            // Count errors in this job
                            let error_count = count_errors_in_job(&job_log);

                            // Parse job status
                            let status = if let Some(job) = github_job {
                                conclusion_to_build_log_status(job.conclusion.as_deref())
                            } else if error_count > 0 {
                                BuildLogJobStatus::Failure
                            } else {
                                BuildLogJobStatus::Success
                            };

                            // Calculate duration
                            let duration = github_job.and_then(|job| {
                                if let (Some(ref started), Some(ref completed)) =
                                    (&job.started_at, &job.completed_at)
                                {
                                    parse_duration(started, completed)
                                } else {
                                    None
                                }
                            });

                            // Build job metadata
                            all_job_metadata.push(BuildLogJobMetadata {
                                name: job_log.name.clone(),
                                workflow_name: workflow_name.clone(),
                                status,
                                error_count,
                                duration,
                                html_url: github_job
                                    .map(|j| j.html_url.clone())
                                    .unwrap_or_default(),
                            });

                            // Convert job_log to JobNode using the parser's built-in function
                            let job_node = gh_actions_log_parser::job_log_to_tree(job_log);
                            workflow_node.total_errors += job_node.error_count;
                            if job_node.error_count > 0 {
                                workflow_node.has_failures = true;
                            }
                            workflow_node.jobs.push(job_node);
                        }

                        all_workflows.push(workflow_node);
                    }
                    Err(e) => {
                        log::warn!("Failed to parse workflow logs for {}: {}", workflow_name, e);
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to download workflow logs for {} (id: {}): {}",
                    workflow_name,
                    workflow_run.id,
                    e
                );
            }
        }
    }

    // Sort workflows: failed first
    all_workflows.sort_by_key(|w| if w.has_failures { 0 } else { 1 });

    Ok((all_workflows, all_job_metadata))
}

/// Count errors in a job log
fn count_errors_in_job(job_log: &gh_actions_log_parser::JobLog) -> usize {
    job_log
        .lines
        .iter()
        .filter(|line| {
            if let Some(ref cmd) = line.command {
                matches!(cmd, gh_actions_log_parser::WorkflowCommand::Error { .. })
            } else {
                line.content.to_lowercase().contains("error:")
            }
        })
        .count()
}

/// Convert GitHub job conclusion to BuildLogJobStatus
fn conclusion_to_build_log_status(conclusion: Option<&str>) -> BuildLogJobStatus {
    match conclusion {
        Some("success") => BuildLogJobStatus::Success,
        Some("failure") => BuildLogJobStatus::Failure,
        Some("cancelled") => BuildLogJobStatus::Cancelled,
        Some("skipped") => BuildLogJobStatus::Skipped,
        None => BuildLogJobStatus::InProgress,
        _ => BuildLogJobStatus::Unknown,
    }
}

/// Parse duration from GitHub API timestamps
fn parse_duration(started: &str, completed: &str) -> Option<Duration> {
    use chrono::DateTime;
    if let (Ok(start), Ok(end)) = (
        DateTime::parse_from_rfc3339(started),
        DateTime::parse_from_rfc3339(completed),
    ) {
        let duration = end.signed_duration_since(start);
        Some(Duration::from_secs(duration.num_seconds().max(0) as u64))
    } else {
        None
    }
}

/// Dispatch CheckBuildStatus actions for the given PRs
fn dispatch_ci_status_checks(
    repo: &Repository,
    prs: &[Pr],
    dispatcher: &Dispatcher,
    _client_manager: Arc<TokioMutex<ClientManager>>,
) {
    for pr in prs {
        dispatcher.dispatch(Action::PullRequest(PullRequestAction::CheckBuildStatus {
            repo: repo.clone(),
            pr_number: pr.number as u64,
            head_sha: pr.head_sha.clone(),
        }));
    }
}

/// Dispatch background fetch for PR stats (additions/deletions)
///
/// The GitHub list PRs endpoint doesn't include additions/deletions, so we
/// need to fetch individual PRs to get these stats.
fn dispatch_pr_stats_fetch(
    repo: &Repository,
    prs: &[Pr],
    dispatcher: &Dispatcher,
    client: CachedGitHubClient<OctocrabClient>,
    _client_manager: Arc<TokioMutex<ClientManager>>,
) {
    for pr in prs {
        let pr_number = pr.number as u64;
        let repo = repo.clone();
        let dispatcher = dispatcher.clone();
        let client = client.clone();

        // Spawn async task for each PR
        tokio::spawn(async move {
            match client
                .fetch_pull_request(&repo.org, &repo.repo, pr_number)
                .await
            {
                Ok(pr_details) => {
                    log::debug!(
                        "Fetched stats for PR #{}: +{} -{}",
                        pr_number,
                        pr_details.additions,
                        pr_details.deletions
                    );
                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::StatsUpdated {
                        repo,
                        pr_number,
                        additions: pr_details.additions as usize,
                        deletions: pr_details.deletions as usize,
                    }));
                }
                Err(e) => {
                    log::warn!("Failed to fetch stats for PR #{}: {}", pr_number, e);
                }
            }
        });
    }
}

/// Fetch PR diff from GitHub API using gh CLI
async fn fetch_pr_diff(
    _octocrab: &Octocrab, // Not used currently, but kept for potential future use
    owner: &str,
    repo: &str,
    pr_number: u64,
    host: Option<&str>,
) -> Result<String, String> {
    // Use gh CLI to fetch the diff with the correct Accept header
    // This is the most reliable way to get the diff in unified format
    let mut args = vec![
        "api".to_string(),
        format!("/repos/{}/{}/pulls/{}", owner, repo, pr_number),
        "-H".to_string(),
        "Accept: application/vnd.github.diff".to_string(),
    ];

    // Add --hostname for GitHub Enterprise hosts
    if let Some(h) = host {
        if h != gh_client::DEFAULT_HOST {
            args.push("--hostname".to_string());
            args.push(h.to_string());
        }
    }

    let output = tokio::process::Command::new("gh")
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to run gh api: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh api failed: {}", stderr));
    }

    let diff_text =
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 in diff: {}", e))?;

    Ok(diff_text)
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
        head_branch: pr.head_branch,
        created_at: pr.created_at,
        updated_at: pr.updated_at,
        html_url: pr.html_url,
        additions: pr.additions as usize,
        deletions: pr.deletions as usize,
    }
}
