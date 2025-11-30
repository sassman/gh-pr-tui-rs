//! GitHub Operations Middleware
//!
//! Central middleware for all GitHub API interactions:
//! - Client initialization (on BootstrapStart)
//! - PR loading (fetch_pull_requests)
//! - PR operations (merge, rebase, approve, close)
//! - CI operations (rerun failed jobs)
//! - Browser/IDE integration

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::domain_models::{MergeableStatus, Pr};
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_client::{
    octocrab::Octocrab, ApiCache, CacheMode, CachedGitHubClient, GitHubClient, MergeMethod,
    OctocrabClient, PullRequest, ReviewEvent,
};
use std::sync::{Arc, Mutex};
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

impl Default for GitHubMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for GitHubMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Initialize client on bootstrap
            Action::BootstrapStart => {
                self.initialize_client();
                true // Let action pass through
            }

            // Handle PR load start - actually fetch the PRs
            Action::PrLoadStart(repo_idx) => self.handle_pr_load(*repo_idx, state, dispatcher, false),

            // Handle PR refresh request (force refresh - bypass cache)
            Action::PrRefresh => {
                if self.client.is_none() {
                    log::warn!("Cannot refresh PRs: GitHub client not initialized");
                    return true;
                }

                let repo_idx = state.main_view.selected_repository;
                self.handle_pr_load(repo_idx, state, dispatcher, true)
            }

            Action::PrOpenInBrowser => {
                let urls = self.get_target_pr_urls(state);
                if urls.is_empty() {
                    log::warn!("No PRs selected for opening in browser");
                    return false;
                }

                log::info!("Opening {} PR(s) in browser", urls.len());

                // Use platform-specific commands (matching gh-pr-tui implementation)
                for url in urls {
                    self.runtime.spawn(async move {
                        #[cfg(target_os = "macos")]
                        let _ = tokio::process::Command::new("open").arg(&url).spawn();

                        #[cfg(target_os = "linux")]
                        let _ = tokio::process::Command::new("xdg-open").arg(&url).spawn();

                        #[cfg(target_os = "windows")]
                        let _ = tokio::process::Command::new("cmd")
                            .args(["/C", "start", &url])
                            .spawn();
                    });
                }
                false // Consume action
            }

            Action::PrMergeRequest => {
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

                        dispatcher.dispatch(Action::PrMergeStart(repo_idx, pr_number));

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
                                    dispatcher
                                        .dispatch(Action::PrMergeSuccess(repo_idx, pr_number));
                                    // Trigger refresh to update PR list
                                    dispatcher.dispatch(Action::PrRefresh);
                                }
                                Ok(result) => {
                                    log::error!("Merge failed: {}", result.message);
                                    dispatcher.dispatch(Action::PrMergeError(
                                        repo_idx,
                                        pr_number,
                                        result.message,
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Merge error: {}", e);
                                    dispatcher.dispatch(Action::PrMergeError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PrRebaseRequest => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
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
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PrRebaseStart(repo_idx, pr_number));

                        self.runtime.spawn(async move {
                            match client
                                .update_pull_request_branch(&owner, &repo, pr_number as u64)
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully rebased PR #{}", pr_number);
                                    dispatcher
                                        .dispatch(Action::PrRebaseSuccess(repo_idx, pr_number));
                                    // Trigger refresh to update PR status
                                    dispatcher.dispatch(Action::PrRefresh);
                                }
                                Err(e) => {
                                    log::error!("Rebase error: {}", e);
                                    dispatcher.dispatch(Action::PrRebaseError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PrApproveRequest => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for approval");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PrApproveStart(repo_idx, pr_number));

                        self.runtime.spawn(async move {
                            match client
                                .create_review(
                                    &owner,
                                    &repo,
                                    pr_number as u64,
                                    ReviewEvent::Approve,
                                    None,
                                )
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully approved PR #{}", pr_number);
                                    dispatcher
                                        .dispatch(Action::PrApproveSuccess(repo_idx, pr_number));
                                }
                                Err(e) => {
                                    log::error!("Approve error: {}", e);
                                    dispatcher.dispatch(Action::PrApproveError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PrCloseRequest => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for closing");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PrCloseStart(repo_idx, pr_number));

                        self.runtime.spawn(async move {
                            match client
                                .close_pull_request(&owner, &repo, pr_number as u64)
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully closed PR #{}", pr_number);
                                    dispatcher
                                        .dispatch(Action::PrCloseSuccess(repo_idx, pr_number));
                                    // Trigger refresh to update PR list
                                    dispatcher.dispatch(Action::PrRefresh);
                                }
                                Err(e) => {
                                    log::error!("Close error: {}", e);
                                    dispatcher.dispatch(Action::PrCloseError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PrOpenBuildLogs => {
                let targets = self.get_target_pr_ci_info(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for opening build logs");
                    return false;
                }

                log::info!("Opening build logs for {} PR(s)", targets.len());

                // Build CI logs URLs and open them
                for (_repo_idx, _pr_number, owner, repo, _head_sha, head_branch) in targets {
                    let url = format!(
                        "https://github.com/{}/{}/actions?query=branch%3A{}",
                        owner, repo, head_branch
                    );

                    // Use platform-specific commands (matching gh-pr-tui implementation)
                    self.runtime.spawn(async move {
                        #[cfg(target_os = "macos")]
                        let _ = tokio::process::Command::new("open").arg(&url).spawn();

                        #[cfg(target_os = "linux")]
                        let _ = tokio::process::Command::new("xdg-open").arg(&url).spawn();

                        #[cfg(target_os = "windows")]
                        let _ = tokio::process::Command::new("cmd")
                            .args(["/C", "start", &url])
                            .spawn();
                    });
                }
                false // Consume action
            }

            Action::PrOpenInIDE => {
                let targets = self.get_target_pr_info_for_ide(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for opening in IDE");
                    return false;
                }

                log::info!("Opening {} PR(s) in IDE", targets.len());

                // Spawn blocking task for each PR to open in IDE
                for (pr_number, org, repo_name) in targets {
                    self.runtime.spawn_blocking(move || {
                        use std::path::PathBuf;
                        use std::process::Command;

                        // Use system temp directory
                        let temp_dir = std::env::temp_dir().join("gh-pr-lander");

                        // Create temp directory if it doesn't exist
                        if let Err(err) = std::fs::create_dir_all(&temp_dir) {
                            log::error!("Failed to create temp directory: {}", err);
                            return;
                        }

                        // Create unique directory for this PR
                        let dir_name = format!("{}-{}-pr-{}", org, repo_name, pr_number);
                        let pr_dir = PathBuf::from(&temp_dir).join(dir_name);

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

                        // Open in IDE (try common IDE commands)
                        // Priority: code (VS Code), cursor, zed, idea, vim
                        let ide_commands = ["code", "cursor", "zed", "idea", "vim"];
                        let mut opened = false;

                        for ide in ide_commands {
                            if Command::new(ide).arg(&pr_dir).spawn().is_ok() {
                                log::info!("Opened PR #{} in {} at {:?}", pr_number, ide, pr_dir);
                                opened = true;
                                break;
                            }
                        }

                        if !opened {
                            log::error!(
                                "Failed to open IDE. Tried: {:?}. PR cloned at: {:?}",
                                ide_commands,
                                pr_dir
                            );
                        }
                    });
                }
                false // Consume action
            }

            Action::PrRerunFailedJobs => {
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
                                        r.conclusion.as_ref().map_or(false, |c| {
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
                                    dispatcher.dispatch(Action::PrRerunStart(
                                        repo_idx,
                                        pr_number,
                                        run.id,
                                    ));

                                    match client.rerun_failed_jobs(&owner, &repo, run.id).await {
                                        Ok(()) => {
                                            log::info!(
                                                "Successfully triggered rerun for workflow {} (PR #{})",
                                                run.name,
                                                pr_number
                                            );
                                            dispatcher.dispatch(Action::PrRerunSuccess(
                                                repo_idx,
                                                pr_number,
                                                run.id,
                                            ));
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "Failed to rerun workflow {} (PR #{}): {}",
                                                run.name,
                                                pr_number,
                                                e
                                            );
                                            dispatcher.dispatch(Action::PrRerunError(
                                                repo_idx,
                                                pr_number,
                                                run.id,
                                                e.to_string(),
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

            _ => true, // Pass through other actions
        }
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
