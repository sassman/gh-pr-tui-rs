//! GitHub Operations Middleware
//!
//! Central middleware for all GitHub API interactions:
//! - Client initialization (on BootstrapStart)
//! - PR loading (fetch_pull_requests)
//! - PR operations (merge, rebase, approve, close)
//! - CI operations (rerun failed jobs)
//! - Browser/IDE integration

use crate::actions::{
    Action, BootstrapAction, BuildLogAction, GlobalAction, PullRequestAction, StatusBarAction,
};
use crate::dispatcher::Dispatcher;
use crate::domain_models::{MergeableStatus, Pr};
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::state::{BuildLogJobMetadata, BuildLogJobStatus, BuildLogPrContext};
use crate::utils::browser::open_url;
use crate::views::BuildLogView;
use gh_client::{
    octocrab::Octocrab, ApiCache, CacheMode, CachedGitHubClient, GitHubClient, MergeMethod,
    OctocrabClient, PullRequest, ReviewEvent,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Middleware for all GitHub API operations
pub struct GitHubMiddleware {
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// Cached GitHub client (initialized on BootstrapStart)
    client: Option<CachedGitHubClient<OctocrabClient>>,
    /// Shared cache instance
    cache: Arc<Mutex<ApiCache>>,
}

impl GitHubMiddleware {
    /// Create a new GitHub middleware
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");

        // Initialize cache from config path
        let cache_file = gh_pr_config::get_cache_file_path()
            .unwrap_or_else(|_| std::env::temp_dir().join("gh-api-cache.json"));
        let cache = Arc::new(Mutex::new(ApiCache::new(cache_file).unwrap_or_default()));

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
                log::info!("GitHubMiddleware: GitHub client initialized with caching");
                self.client = Some(client);
            }
            Err(e) => {
                log::warn!("GitHubMiddleware: GitHub client not initialized: {}", e);
            }
        }
    }

    /// Get a client configured for force refresh (WriteOnly mode)
    fn force_refresh_client(&self) -> Option<CachedGitHubClient<OctocrabClient>> {
        self.client
            .as_ref()
            .map(|c| c.with_mode(CacheMode::WriteOnly))
    }

    /// Get a cloneable octocrab instance for async operations
    fn octocrab_arc(&self) -> Option<Arc<Octocrab>> {
        self.client.as_ref().map(|c| c.inner().octocrab_arc())
    }

    /// Get target PRs for an operation (selected PRs or cursor PR)
    fn get_target_prs(&self, state: &AppState) -> Vec<(usize, usize)> {
        let repo_idx = state.main_view.selected_repository;

        if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
            // If there are selected PRs, use those
            if !repo_data.selected_pr_numbers.is_empty() {
                return repo_data
                    .selected_pr_numbers
                    .iter()
                    .map(|&pr_num| (repo_idx, pr_num))
                    .collect();
            }

            // Otherwise use the cursor PR
            if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                return vec![(repo_idx, pr.number)];
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
    /// Returns: Vec<(pr_number, repo_org, repo_name)>
    fn get_target_pr_info_for_ide(&self, state: &AppState) -> Vec<(usize, String, String)> {
        let repo_idx = state.main_view.selected_repository;

        if let Some(repo) = state.main_view.repositories.get(repo_idx) {
            if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
                // If there are selected PRs, use those
                if !repo_data.selected_pr_numbers.is_empty() {
                    return repo_data
                        .selected_pr_numbers
                        .iter()
                        .map(|&pr_num| (pr_num, repo.org.clone(), repo.repo.clone()))
                        .collect();
                }

                // Otherwise use the cursor PR
                if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                    return vec![(pr.number, repo.org.clone(), repo.repo.clone())];
                }
            }
        }

        vec![]
    }

    /// Get target PR CI info for build operations (respects multi-selection)
    /// Returns: Vec<(repo_idx, pr_number, owner, repo, head_sha, head_branch)>
    fn get_target_pr_ci_info(
        &self,
        state: &AppState,
    ) -> Vec<(usize, u64, String, String, String, String)> {
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
                                repo_idx,
                                pr.number as u64,
                                repo.org.clone(),
                                repo.repo.clone(),
                                pr.head_sha.clone(),
                                pr.head_branch.clone(),
                            )
                        })
                        .collect();
                }

                // Otherwise use the cursor PR
                if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                    return vec![(
                        repo_idx,
                        pr.number as u64,
                        repo.org.clone(),
                        repo.repo.clone(),
                        pr.head_sha.clone(),
                        pr.head_branch.clone(),
                    )];
                }
            }
        }

        vec![]
    }

    /// Get repository info for a PR operation
    fn get_repo_info(&self, state: &AppState, repo_idx: usize) -> Option<(String, String)> {
        state
            .main_view
            .repositories
            .get(repo_idx)
            .map(|r| (r.org.clone(), r.repo.clone()))
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

        let client = if force_refresh {
            let Some(client) = self.force_refresh_client() else {
                log::error!("PrLoad: client not initialized");
                dispatcher.dispatch(Action::PullRequest(PullRequestAction::LoadError(
                    repo_idx,
                    "GitHub client not initialized".to_string(),
                )));
                return true;
            };
            client
        } else {
            let Some(client) = self.client.clone() else {
                log::error!("PrLoad: client not initialized");
                dispatcher.dispatch(Action::PullRequest(PullRequestAction::LoadError(
                    repo_idx,
                    "GitHub client not initialized".to_string(),
                )));
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
        let mode = if force_refresh {
            "force refresh"
        } else {
            "cached"
        };
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
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::info(
                        format!("Loaded {} PRs from {}/{}", domain_prs.len(), org, repo_name),
                        "Load",
                    )));
                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::Loaded(
                        repo_idx, domain_prs,
                    )));
                }
                Err(e) => {
                    log::error!("Failed to load PRs for {}/{}: {}", org, repo_name, e);
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                        format!("Failed to load PRs: {}", e),
                        "Load",
                    )));
                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::LoadError(
                        repo_idx,
                        e.to_string(),
                    )));
                }
            }
        });

        true // Let action pass through to reducer (to set loading state)
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
            // Initialize client on bootstrap
            Action::Bootstrap(BootstrapAction::Start) => {
                self.initialize_client();
                true // Let action pass through
            }

            // Handle PR load start - actually fetch the PRs
            Action::PullRequest(PullRequestAction::LoadStart(repo_idx)) => {
                self.handle_pr_load(*repo_idx, state, dispatcher, false)
            }

            // Handle PR refresh request (force refresh - bypass cache)
            Action::PullRequest(PullRequestAction::Refresh) => {
                if self.client.is_none() {
                    log::warn!("Cannot refresh PRs: GitHub client not initialized");
                    return true;
                }

                let repo_idx = state.main_view.selected_repository;
                self.handle_pr_load(repo_idx, state, dispatcher, true)
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
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for merge");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PullRequest(PullRequestAction::MergeStart(
                            repo_idx, pr_number,
                        )));
                        dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                            format!("Merging PR #{}...", pr_number),
                            "Merge",
                        )));

                        self.runtime.spawn(async move {
                            match client
                                .merge_pull_request(
                                    &owner,
                                    &repo,
                                    pr_number as u64,
                                    MergeMethod::default(),
                                    None,
                                    None,
                                )
                                .await
                            {
                                Ok(result) if result.merged => {
                                    log::info!("Successfully merged PR #{}", pr_number);
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!("PR #{} merged", pr_number),
                                            "Merge",
                                        ),
                                    ));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::MergeSuccess(repo_idx, pr_number),
                                    ));
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
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::MergeError(
                                            repo_idx,
                                            pr_number,
                                            result.message,
                                        ),
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Merge error: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Merge error: {}", e),
                                        "Merge",
                                    )));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::MergeError(
                                            repo_idx,
                                            pr_number,
                                            e.to_string(),
                                        ),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::RebaseRequest) => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let octocrab = match self.octocrab_arc() {
                    Some(c) => c,
                    None => {
                        log::error!("Octocrab client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for rebase");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        // Look up PR author to determine rebase method
                        let is_dependabot = state
                            .main_view
                            .repo_data
                            .get(&repo_idx)
                            .and_then(|data| data.prs.iter().find(|pr| pr.number == pr_number))
                            .map(|pr| pr.author.to_lowercase().contains("dependabot"))
                            .unwrap_or(false);

                        let dispatcher = dispatcher.clone();

                        dispatcher.dispatch(Action::PullRequest(PullRequestAction::RebaseStart(
                            repo_idx, pr_number,
                        )));
                        dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                            format!("Updating branch for PR #{}...", pr_number),
                            "Rebase",
                        )));

                        if is_dependabot {
                            // For dependabot PRs, post a comment to trigger rebase
                            let octocrab = Arc::clone(&octocrab);
                            self.runtime.spawn(async move {
                                match octocrab
                                    .issues(&owner, &repo)
                                    .create_comment(pr_number as u64, "@dependabot rebase")
                                    .await
                                {
                                    Ok(_) => {
                                        log::info!(
                                            "Requested dependabot rebase for PR #{}",
                                            pr_number
                                        );
                                        dispatcher.dispatch(Action::StatusBar(
                                            StatusBarAction::success(
                                                format!(
                                                    "Dependabot rebase requested for PR #{}",
                                                    pr_number
                                                ),
                                                "Rebase",
                                            ),
                                        ));
                                        dispatcher.dispatch(Action::PullRequest(
                                            PullRequestAction::RebaseSuccess(repo_idx, pr_number),
                                        ));
                                    }
                                    Err(e) => {
                                        log::error!("Dependabot rebase request error: {}", e);
                                        dispatcher.dispatch(Action::StatusBar(
                                            StatusBarAction::error(
                                                format!("Rebase request failed: {}", e),
                                                "Rebase",
                                            ),
                                        ));
                                        dispatcher.dispatch(Action::PullRequest(
                                            PullRequestAction::RebaseError(
                                                repo_idx,
                                                pr_number,
                                                e.to_string(),
                                            ),
                                        ));
                                    }
                                }
                            });
                        } else {
                            // For regular PRs, use the update branch API
                            let client = client.clone();
                            self.runtime.spawn(async move {
                                match client
                                    .update_pull_request_branch(&owner, &repo, pr_number as u64)
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
                                        dispatcher.dispatch(Action::PullRequest(
                                            PullRequestAction::RebaseSuccess(repo_idx, pr_number),
                                        ));
                                        // Trigger refresh to update PR status
                                        dispatcher.dispatch(Action::PullRequest(
                                            PullRequestAction::Refresh,
                                        ));
                                    }
                                    Err(e) => {
                                        log::error!("Rebase error: {}", e);
                                        dispatcher.dispatch(Action::StatusBar(
                                            StatusBarAction::error(
                                                format!("Rebase failed: {}", e),
                                                "Rebase",
                                            ),
                                        ));
                                        dispatcher.dispatch(Action::PullRequest(
                                            PullRequestAction::RebaseError(
                                                repo_idx,
                                                pr_number,
                                                e.to_string(),
                                            ),
                                        ));
                                    }
                                }
                            });
                        }
                    }
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
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let repo_idx = state.main_view.selected_repository;
                let message = if message.is_empty() {
                    None
                } else {
                    Some(message.clone())
                };

                for pr_number in pr_numbers {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();
                        let message = message.clone();
                        let pr_num = *pr_number as usize;
                        let pr_number_owned = *pr_number; // Clone to owned value

                        dispatcher.dispatch(Action::PullRequest(PullRequestAction::ApproveStart(
                            repo_idx, pr_num,
                        )));
                        dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                            format!("Approving PR #{}...", pr_number_owned),
                            "Approve",
                        )));

                        self.runtime.spawn(async move {
                            match client
                                .create_review(
                                    &owner,
                                    &repo,
                                    pr_number_owned,
                                    ReviewEvent::Approve,
                                    message.as_deref(),
                                )
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully approved PR #{}", pr_number_owned);
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!("PR #{} approved", pr_number_owned),
                                            "Approve",
                                        ),
                                    ));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::ApproveSuccess(repo_idx, pr_num),
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Approve error: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Approve failed: {}", e),
                                        "Approve",
                                    )));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::ApproveError(
                                            repo_idx,
                                            pr_num,
                                            e.to_string(),
                                        ),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::CommentOnPr {
                pr_numbers,
                message,
            }) => {
                let client = match self.octocrab_arc() {
                    Some(c) => c,
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let repo_idx = state.main_view.selected_repository;

                for pr_number in pr_numbers {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = Arc::clone(&client);
                        let message = message.clone();
                        let pr_num = *pr_number as usize;
                        let pr_number_owned = *pr_number; // Clone to owned value

                        dispatcher.dispatch(Action::PullRequest(PullRequestAction::CommentStart(
                            repo_idx, pr_num,
                        )));
                        dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                            format!("Commenting on PR #{}...", pr_number_owned),
                            "Comment",
                        )));

                        self.runtime.spawn(async move {
                            match client
                                .issues(&owner, &repo)
                                .create_comment(pr_number_owned, &message)
                                .await
                            {
                                Ok(_) => {
                                    log::info!("Successfully commented on PR #{}", pr_number_owned);
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!("Commented on PR #{}", pr_number_owned),
                                            "Comment",
                                        ),
                                    ));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::CommentSuccess(repo_idx, pr_num),
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Comment error: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Comment failed: {}", e),
                                        "Comment",
                                    )));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::CommentError(
                                            repo_idx,
                                            pr_num,
                                            e.to_string(),
                                        ),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PullRequest(PullRequestAction::RequestChanges {
                pr_numbers,
                message,
            }) => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let repo_idx = state.main_view.selected_repository;

                for pr_number in pr_numbers {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();
                        let message = message.clone();
                        let pr_num = *pr_number as usize;
                        let pr_number_owned = *pr_number; // Clone to owned value

                        dispatcher.dispatch(Action::PullRequest(
                            PullRequestAction::RequestChangesStart(repo_idx, pr_num),
                        ));
                        dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                            format!("Requesting changes on PR #{}...", pr_number_owned),
                            "Request Changes",
                        )));

                        self.runtime.spawn(async move {
                            match client
                                .create_review(
                                    &owner,
                                    &repo,
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
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!("Requested changes on PR #{}", pr_number_owned),
                                            "Request Changes",
                                        ),
                                    ));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::RequestChangesSuccess(repo_idx, pr_num),
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Request changes error: {}", e);
                                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                                        format!("Request changes failed: {}", e),
                                        "Request Changes",
                                    )));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::RequestChangesError(
                                            repo_idx,
                                            pr_num,
                                            e.to_string(),
                                        ),
                                    ));
                                }
                            }
                        });
                    }
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
                let client = match self.octocrab_arc() {
                    Some(c) => c,
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let cached_client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let repo_idx = state.main_view.selected_repository;

                for pr_number in pr_numbers {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = Arc::clone(&client);
                        let cached_client = cached_client.clone();
                        let message = message.clone();
                        let pr_num = *pr_number as usize;
                        let pr_number_owned = *pr_number; // Clone to owned value

                        dispatcher.dispatch(Action::PullRequest(PullRequestAction::CloseStart(
                            repo_idx, pr_num,
                        )));
                        dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                            format!("Closing PR #{}...", pr_number_owned),
                            "Close",
                        )));

                        self.runtime.spawn(async move {
                            // Post comment if message is not empty
                            if !message.is_empty() {
                                if let Err(e) = client
                                    .issues(&owner, &repo)
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
                            match cached_client
                                .close_pull_request(&owner, &repo, pr_number_owned)
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully closed PR #{}", pr_number_owned);
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!("PR #{} closed", pr_number_owned),
                                            "Close",
                                        ),
                                    ));
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::CloseSuccess(repo_idx, pr_num),
                                    ));
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
                                    dispatcher.dispatch(Action::PullRequest(
                                        PullRequestAction::CloseError(
                                            repo_idx,
                                            pr_num,
                                            e.to_string(),
                                        ),
                                    ));
                                }
                            }
                        });
                    }
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

                for (_repo_idx, _pr_number, owner, repo, _head_sha, head_branch) in targets {
                    let url = format!(
                        "https://github.com/{}/{}/actions?query=branch%3A{}",
                        owner, repo, head_branch
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
                for (pr_number, org, repo_name) in targets {
                    let ide_command = ide_command.clone();
                    let temp_dir_base = temp_dir_base.clone();

                    self.runtime.spawn_blocking(move || {
                        use std::path::PathBuf;
                        use std::process::Command;

                        let temp_dir = PathBuf::from(&temp_dir_base);

                        // Create temp directory if it doesn't exist
                        if let Err(err) = std::fs::create_dir_all(&temp_dir) {
                            log::error!("Failed to create temp directory: {}", err);
                            return;
                        }

                        // Create unique directory for this PR
                        let dir_name = format!("{}-{}-pr-{}", org, repo_name, pr_number);
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
                        let clone_output = Command::new("gh")
                            .args([
                                "repo",
                                "clone",
                                &format!("{}/{}", org, repo_name),
                                &pr_dir.to_string_lossy(),
                            ])
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
                        let ssh_url = format!("git@github.com:{}/{}.git", org, repo_name);
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
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_pr_ci_info(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for rerunning jobs");
                    return false;
                }

                log::info!("Rerunning failed jobs for {} PR(s)", targets.len());

                // Rerun failed jobs for each target PR
                for (repo_idx, pr_number, owner, repo, head_sha, _head_branch) in targets {
                    let dispatcher = dispatcher.clone();
                    let client = client.clone();

                    // Fetch workflow runs, then rerun failed ones
                    self.runtime.spawn(async move {
                        // Fetch workflow runs for this commit
                        match client.fetch_workflow_runs(&owner, &repo, &head_sha).await {
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
                                    dispatcher.dispatch(Action::PullRequest(PullRequestAction::RerunStart(
                                        repo_idx,
                                        pr_number,
                                        run.id,
                                    )));

                                    match client.rerun_failed_jobs(&owner, &repo, run.id).await {
                                        Ok(()) => {
                                            log::info!(
                                                "Successfully triggered rerun for workflow {} (PR #{})",
                                                run.name,
                                                pr_number
                                            );
                                            dispatcher.dispatch(Action::PullRequest(PullRequestAction::RerunSuccess(
                                                repo_idx,
                                                pr_number,
                                                run.id,
                                            )));
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "Failed to rerun workflow {} (PR #{}): {}",
                                                run.name,
                                                pr_number,
                                                e
                                            );
                                            dispatcher.dispatch(Action::PullRequest(PullRequestAction::RerunError(
                                                repo_idx,
                                                pr_number,
                                                run.id,
                                                e.to_string(),
                                            )));
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
                let Some(repo) = state.main_view.repositories.get(repo_idx) else {
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

                // Get octocrab client
                let Some(octocrab) = self.octocrab_arc() else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                        "GitHub client not initialized",
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
        head_branch: pr.head_branch,
        created_at: pr.created_at,
        updated_at: pr.updated_at,
        html_url: pr.html_url,
    }
}
