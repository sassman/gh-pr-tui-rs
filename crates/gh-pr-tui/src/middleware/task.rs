//! TaskMiddleware - handles async operations like loading repos, merging PRs, etc.

use super::{BoxFuture, Dispatcher, Middleware};
use crate::{actions::Action, state::AppState};

/// TaskMiddleware - handles async operations like loading repos, merging PRs, etc.
///
/// This middleware replaces the old Effect/BackgroundTask system by handling
/// async operations directly in response to actions.
///
/// # Example Operations
/// - Bootstrap → load .env, init Octocrab, load repos → dispatch BootstrapComplete
/// - RefreshCurrentRepo → fetch PR data from GitHub → dispatch RepoDataLoaded
/// - MergeSelectedPrs → call GitHub API → dispatch MergeComplete
/// - Rebase → call GitHub API → dispatch RebaseComplete
///
/// # Design
/// The middleware spawns tokio tasks for async operations and dispatches
/// result actions when complete. This eliminates the need for:
/// - Effect enum
/// - BackgroundTask enum
/// - TaskResult enum
/// - result_to_action() conversion
pub struct TaskMiddleware {
    /// GitHub client (set after initialization)
    octocrab: Option<octocrab::Octocrab>,
    /// API response cache
    cache: std::sync::Arc<std::sync::Mutex<gh_api_cache::ApiCache>>,
}

impl TaskMiddleware {
    pub fn new(
        cache: std::sync::Arc<std::sync::Mutex<gh_api_cache::ApiCache>>,
    ) -> Self {
        Self {
            octocrab: None,
            cache,
        }
    }

    /// Get octocrab client (returns error if not initialized)
    fn octocrab(&self) -> Result<octocrab::Octocrab, String> {
        self.octocrab
            .clone()
            .ok_or_else(|| "Octocrab not initialized".to_string())
    }
}

impl Middleware for TaskMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            use crate::actions::{Action, BootstrapResult};
            use crate::state::{TaskStatus, TaskStatusType};

            match action {
                //
                // BOOTSTRAP FLOW
                //

                Action::Bootstrap => {
                    log::debug!("TaskMiddleware: Handling Bootstrap");

                    // Step 1: Load .env file if GITHUB_TOKEN not set
                    if std::env::var("GITHUB_TOKEN").is_err() {
                        match dotenvy::dotenv() {
                            Ok(path) => {
                                log::debug!("Loaded .env file from: {:?}", path);
                            }
                            Err(_) => {
                                log::debug!(".env file not found, will rely on environment variables");
                            }
                        }
                    }

                    // Step 2: Initialize Octocrab
                    match std::env::var("GITHUB_TOKEN") {
                        Ok(token) => match octocrab::Octocrab::builder()
                            .personal_token(token)
                            .build()
                        {
                            Ok(client) => {
                                log::debug!("Octocrab client initialized successfully");
                                dispatcher.dispatch(Action::OctocrabInitialized(client));
                            }
                            Err(e) => {
                                log::error!("Failed to initialize octocrab: {}", e);
                                dispatcher.dispatch(Action::BootstrapComplete(Err(format!(
                                    "Failed to initialize GitHub client: {}",
                                    e
                                ))));
                                return true; // Stop bootstrap flow
                            }
                        },
                        Err(_) => {
                            dispatcher.dispatch(Action::BootstrapComplete(Err(
                                "GITHUB_TOKEN environment variable not set. Please set it or create a .env file.".to_string()
                            )));
                            return true; // Stop bootstrap flow
                        }
                    }
                }

                Action::OctocrabInitialized(client) => {
                    log::debug!("TaskMiddleware: Storing Octocrab client");
                    // Store the client for future use
                    self.octocrab = Some(client.clone());

                    // Step 3: Load repositories from config
                    match crate::loading_recent_repos() {
                        Ok(repos) => {
                            if repos.is_empty() {
                                dispatcher.dispatch(Action::BootstrapComplete(Err(
                                    "No repositories configured. Add repositories to .recent-repositories.json".to_string()
                                )));
                                return true;
                            }

                            // Restore session
                            let selected_repo: usize =
                                if let Ok(persisted_state) = crate::load_persisted_state() {
                                    repos
                                        .iter()
                                        .position(|r| r == &persisted_state.selected_repo)
                                        .unwrap_or_default()
                                } else {
                                    0
                                };

                            // Dispatch bootstrap complete
                            let result = BootstrapResult {
                                repos,
                                selected_repo,
                            };
                            dispatcher.dispatch(Action::BootstrapComplete(Ok(result)));
                        }
                        Err(err) => {
                            dispatcher.dispatch(Action::BootstrapComplete(Err(err.to_string())));
                        }
                    }
                }

                //
                // REPO LOADING OPERATIONS
                //

                Action::RefreshCurrentRepo => {
                    log::debug!("TaskMiddleware: Handling RefreshCurrentRepo");

                    // Get current repo info
                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        let filter = state.repos.filter.clone();

                        // Dispatch loading status
                        dispatcher.dispatch(Action::SetReposLoading(vec![repo_index]));
                        dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                            message: "Refreshing...".to_string(),
                            status_type: TaskStatusType::Running,
                        })));

                        // Spawn background task directly
                        if let Ok(octocrab) = self.octocrab() {
                            let cache = self.cache.clone();
                            let dispatcher = dispatcher.clone();
                            tokio::spawn(async move {
                                log::debug!(
                                    "Loading repo {}/{} (index: {}, bypass_cache: true)...",
                                    repo.org, repo.repo, repo_index
                                );
                                let result = crate::fetch_github_data_cached(
                                    &octocrab,
                                    &repo,
                                    &filter,
                                    &cache,
                                    true, // Refresh always bypasses cache
                                )
                                .await
                                .map_err(|e| e.to_string());

                                // Log success or error
                                match &result {
                                    Ok(prs) => {
                                        log::debug!(
                                            "Successfully loaded {}/{}: {} PRs",
                                            repo.org,
                                            repo.repo,
                                            prs.len()
                                        );
                                    }
                                    Err(err) => {
                                        log::error!("Failed to load {}/{}: {}", repo.org, repo.repo, err);
                                    }
                                }

                                dispatcher.dispatch(Action::RepoDataLoaded(repo_index, result));
                            });
                        }
                    }
                }

                Action::ReloadRepo(repo_index) => {
                    log::debug!("TaskMiddleware: Handling ReloadRepo {}", repo_index);

                    if let Some(repo) = state.repos.recent_repos.get(*repo_index).cloned() {
                        let filter = state.repos.filter.clone();

                        // Dispatch loading status
                        dispatcher.dispatch(Action::SetReposLoading(vec![*repo_index]));

                        // Spawn background task directly
                        if let Ok(octocrab) = self.octocrab() {
                            let cache = self.cache.clone();
                            let dispatcher = dispatcher.clone();
                            let repo_index = *repo_index; // Copy for async move
                            tokio::spawn(async move {
                                log::debug!(
                                    "Loading repo {}/{} (index: {}, bypass_cache: false)...",
                                    repo.org, repo.repo, repo_index
                                );
                                let result = crate::fetch_github_data_cached(
                                    &octocrab,
                                    &repo,
                                    &filter,
                                    &cache,
                                    false, // Normal reload uses cache
                                )
                                .await
                                .map_err(|e| e.to_string());

                                // Log success or error
                                match &result {
                                    Ok(prs) => {
                                        log::debug!(
                                            "Successfully loaded {}/{}: {} PRs",
                                            repo.org,
                                            repo.repo,
                                            prs.len()
                                        );
                                    }
                                    Err(err) => {
                                        log::error!("Failed to load {}/{}: {}", repo.org, repo.repo, err);
                                    }
                                }

                                dispatcher.dispatch(Action::RepoDataLoaded(repo_index, result));
                            });
                        }
                    }
                }

                //
                // SIMPLE OPERATIONS
                //

                Action::OpenCurrentPrInBrowser => {
                    log::debug!("TaskMiddleware: Handling OpenCurrentPrInBrowser");

                    // Get current repo and selected PRs
                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index) {
                        // Check if there are selected PRs
                        let has_selection = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            !repo_data.selected_pr_numbers.is_empty()
                        } else {
                            false
                        };

                        // Get PR numbers to open
                        let prs_to_open: Vec<usize> = if has_selection {
                            if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                repo_data
                                    .prs
                                    .iter()
                                    .filter(|pr| {
                                        repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                    })
                                    .map(|pr| pr.number)
                                    .collect()
                            } else {
                                Vec::new()
                            }
                        } else if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            // No selection, open just the currently focused PR
                            if let Some(selected_idx) = repo_data.table_state.selected() {
                                repo_data
                                    .prs
                                    .get(selected_idx)
                                    .map(|pr| vec![pr.number])
                                    .unwrap_or_default()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        };

                        // Open each PR in browser
                        for pr_number in prs_to_open {
                            let url = format!(
                                "https://github.com/{}/{}/pull/{}",
                                repo.org, repo.repo, pr_number
                            );
                            log::debug!("Opening in browser: {}", url);

                            // Spawn async task to open URL (platform-specific)
                            tokio::spawn(async move {
                                #[cfg(target_os = "macos")]
                                let _ = tokio::process::Command::new("open")
                                    .arg(&url)
                                    .spawn();

                                #[cfg(target_os = "linux")]
                                let _ = tokio::process::Command::new("xdg-open")
                                    .arg(&url)
                                    .spawn();

                                #[cfg(target_os = "windows")]
                                let _ = tokio::process::Command::new("cmd")
                                    .args(["/C", "start", &url])
                                    .spawn();
                            });
                        }
                    }
                }

                Action::OpenInIDE => {
                    log::debug!("TaskMiddleware: Handling OpenInIDE");

                    // Get current repo and selected PR
                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        let config = state.config.clone();

                        if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            // Check if a PR is selected
                            let pr_number = if let Some(selected_idx) = repo_data.table_state.selected() {
                                repo_data.prs.get(selected_idx).map(|pr| pr.number).unwrap_or(0)
                            } else {
                                // No PR selected (empty list) - open main branch
                                0
                            };

                            // Set status message
                            let message = if pr_number == 0 {
                                "Opening main branch in IDE...".to_string()
                            } else {
                                format!("Opening PR #{} in IDE...", pr_number)
                            };
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message,
                                status_type: TaskStatusType::Running,
                            })));

                            // Spawn blocking task to open in IDE (using blocking commands)
                            let dispatcher = dispatcher.clone();
                            let ide_command = config.ide_command;
                            let temp_dir = config.temp_dir;
                            tokio::task::spawn_blocking(move || {
                                use std::path::PathBuf;
                                use std::process::Command;

                                // Create temp directory if it doesn't exist
                                if let Err(err) = std::fs::create_dir_all(&temp_dir) {
                                    dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                        "Failed to create temp directory: {}",
                                        err
                                    ))));
                                    return;
                                }

                                // Create unique directory for this PR or main branch
                                let dir_name = if pr_number == 0 {
                                    format!("{}-{}-main", repo.org, repo.repo)
                                } else {
                                    format!("{}-{}-pr-{}", repo.org, repo.repo, pr_number)
                                };
                                let pr_dir = PathBuf::from(&temp_dir).join(dir_name);

                                // Remove existing directory if present
                                if pr_dir.exists()
                                    && let Err(err) = std::fs::remove_dir_all(&pr_dir)
                                {
                                    dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                        "Failed to remove existing directory: {}",
                                        err
                                    ))));
                                    return;
                                }

                                // Clone the repository using gh repo clone (uses SSH by default)
                                let clone_output = Command::new("gh")
                                    .args([
                                        "repo",
                                        "clone",
                                        &format!("{}/{}", repo.org, repo.repo),
                                        &pr_dir.to_string_lossy(),
                                    ])
                                    .output();

                                if let Err(err) = clone_output {
                                    dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                        "Failed to run gh repo clone: {}",
                                        err
                                    ))));
                                    return;
                                }

                                let clone_output = clone_output.unwrap();
                                if !clone_output.status.success() {
                                    let stderr = String::from_utf8_lossy(&clone_output.stderr);
                                    dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                        "gh repo clone failed: {}",
                                        stderr
                                    ))));
                                    return;
                                }

                                // Checkout PR branch or main branch
                                if pr_number == 0 {
                                    // Checkout main branch and pull latest
                                    let checkout_output = Command::new("git")
                                        .args(["checkout", "main"])
                                        .current_dir(&pr_dir)
                                        .output();

                                    if let Err(err) = checkout_output {
                                        dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                            "Failed to run git checkout main: {}",
                                            err
                                        ))));
                                        return;
                                    }

                                    let checkout_output = checkout_output.unwrap();
                                    if !checkout_output.status.success() {
                                        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
                                        dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                            "git checkout main failed: {}",
                                            stderr
                                        ))));
                                        return;
                                    }

                                    // Pull latest changes
                                    let pull_output = Command::new("git")
                                        .args(["pull"])
                                        .current_dir(&pr_dir)
                                        .output();

                                    if let Err(err) = pull_output {
                                        dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                            "Failed to run git pull: {}",
                                            err
                                        ))));
                                        return;
                                    }

                                    let pull_output = pull_output.unwrap();
                                    if !pull_output.status.success() {
                                        let stderr = String::from_utf8_lossy(&pull_output.stderr);
                                        dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                            "git pull failed: {}",
                                            stderr
                                        ))));
                                        return;
                                    }
                                } else {
                                    // Checkout the PR using gh pr checkout
                                    let checkout_output = Command::new("gh")
                                        .args(["pr", "checkout", &pr_number.to_string()])
                                        .current_dir(&pr_dir)
                                        .output();

                                    if let Err(err) = checkout_output {
                                        dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                            "Failed to run gh pr checkout: {}",
                                            err
                                        ))));
                                        return;
                                    }

                                    let checkout_output = checkout_output.unwrap();
                                    if !checkout_output.status.success() {
                                        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
                                        dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                            "gh pr checkout failed: {}",
                                            stderr
                                        ))));
                                        return;
                                    }
                                }

                                // Set origin URL to SSH (gh checkout doesn't do this)
                                let ssh_url = format!("git@github.com:{}/{}.git", repo.org, repo.repo);
                                let set_url_output = Command::new("git")
                                    .args(["remote", "set-url", "origin", &ssh_url])
                                    .current_dir(&pr_dir)
                                    .output();

                                if let Err(err) = set_url_output {
                                    dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                        "Failed to set SSH origin URL: {}",
                                        err
                                    ))));
                                    return;
                                }

                                let set_url_output = set_url_output.unwrap();
                                if !set_url_output.status.success() {
                                    let stderr = String::from_utf8_lossy(&set_url_output.stderr);
                                    dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                        "Failed to set SSH origin URL: {}",
                                        stderr
                                    ))));
                                    return;
                                }

                                // Open in IDE
                                let ide_output = Command::new(&ide_command).arg(&pr_dir).spawn();

                                match ide_output {
                                    Ok(_) => {
                                        dispatcher.dispatch(Action::IDEOpenComplete(Ok(())));
                                    }
                                    Err(err) => {
                                        dispatcher.dispatch(Action::IDEOpenComplete(Err(format!(
                                            "Failed to open IDE '{}': {}",
                                            ide_command, err
                                        ))));
                                    }
                                }
                            });
                        }
                    }
                }

                Action::AddRepoFormSubmit => {
                    log::debug!("TaskMiddleware: Handling AddRepoFormSubmit");

                    // Build the new repo from form data
                    let branch = if state.ui.add_repo_form.branch.is_empty() {
                        "main".to_string()
                    } else {
                        state.ui.add_repo_form.branch.clone()
                    };

                    let new_repo = crate::state::Repo {
                        org: state.ui.add_repo_form.org.clone(),
                        repo: state.ui.add_repo_form.repo.clone(),
                        branch,
                    };

                    // Check if repository already exists
                    let repo_exists = state
                        .repos
                        .recent_repos
                        .iter()
                        .any(|r| {
                            r.org == new_repo.org
                                && r.repo == new_repo.repo
                                && r.branch == new_repo.branch
                        });

                    if !repo_exists {
                        // Calculate new repo index
                        let repo_index = state.repos.recent_repos.len();

                        // Build new repos list for saving
                        let mut new_repos = state.repos.recent_repos.clone();
                        new_repos.push(new_repo.clone());

                        // Save to file asynchronously
                        let dispatcher = dispatcher.clone();
                        let new_repo_for_action = new_repo.clone();
                        tokio::spawn(async move {
                            match crate::store_recent_repos(&new_repos) {
                                Ok(_) => {
                                    // Dispatch success actions
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: format!(
                                            "Repository {}/{} added",
                                            new_repo.org, new_repo.repo
                                        ),
                                        status_type: TaskStatusType::Success,
                                    })));
                                    dispatcher.dispatch(Action::RepositoryAdded {
                                        repo_index,
                                        repo: new_repo_for_action.clone(),
                                    });
                                    dispatcher.dispatch(Action::SelectRepoByIndex(repo_index));
                                    dispatcher.dispatch(Action::ReloadRepo(repo_index));
                                }
                                Err(e) => {
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: format!("Failed to save repository: {}", e),
                                        status_type: TaskStatusType::Error,
                                    })));
                                }
                            }
                        });
                    } else {
                        // Repository already exists
                        dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                            message: format!(
                                "Repository {}/{} already exists",
                                new_repo.org, new_repo.repo
                            ),
                            status_type: TaskStatusType::Error,
                        })));
                    }
                }

                Action::DeleteCurrentRepo => {
                    log::debug!("TaskMiddleware: Handling DeleteCurrentRepo");

                    // Build updated repos list without current repo
                    let repo_index = state.repos.selected_repo;
                    let mut new_repos = state.repos.recent_repos.clone();

                    if repo_index < new_repos.len() {
                        new_repos.remove(repo_index);

                        // Save to file asynchronously
                        let dispatcher = dispatcher.clone();
                        tokio::spawn(async move {
                            match crate::store_recent_repos(&new_repos) {
                                Ok(_) => {
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: "Repository deleted".to_string(),
                                        status_type: TaskStatusType::Success,
                                    })));
                                }
                                Err(e) => {
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: format!("Failed to save repositories: {}", e),
                                        status_type: TaskStatusType::Error,
                                    })));
                                }
                            }
                        });
                    }
                }

                //
                // PR OPERATIONS
                //

                Action::MergeSelectedPrs => {
                    log::debug!("TaskMiddleware: Handling MergeSelectedPrs");

                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        // Check if there are selected PRs
                        let has_selection = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            !repo_data.selected_pr_numbers.is_empty()
                        } else {
                            false
                        };

                        // Get PRs to merge
                        let selected_prs: Vec<_> = if !has_selection {
                            // No selection - use current cursor PR
                            if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                if let Some(selected_idx) = repo_data.table_state.selected() {
                                    repo_data.prs.get(selected_idx).cloned().map(|pr| vec![pr]).unwrap_or_default()
                                } else {
                                    vec![]
                                }
                            } else {
                                vec![]
                            }
                        } else if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            repo_data
                                .prs
                                .iter()
                                .filter(|pr| {
                                    repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                })
                                .cloned()
                                .collect()
                        } else {
                            vec![]
                        };

                        if !selected_prs.is_empty() {
                            // Separate PRs by status: ready to merge vs building
                            let mut prs_to_merge = Vec::new();
                            let mut prs_to_auto_merge = Vec::new();

                            for pr in selected_prs {
                                match pr.mergeable {
                                    crate::pr::MergeableStatus::BuildInProgress => {
                                        prs_to_auto_merge.push(pr);
                                    }
                                    _ => {
                                        prs_to_merge.push(pr);
                                    }
                                }
                            }

                            // Merge ready PRs directly
                            if !prs_to_merge.is_empty() {
                                // Start monitoring for each PR being merged
                                for pr in &prs_to_merge {
                                    dispatcher.dispatch(Action::StartOperationMonitor(
                                        repo_index,
                                        pr.number,
                                        crate::state::OperationType::Merge,
                                    ));
                                }

                                // Set task status
                                dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                    message: format!("Merging {} PR(s)...", prs_to_merge.len()),
                                    status_type: TaskStatusType::Running,
                                })));

                                // Spawn async task to merge PRs
                                let selected_indices: Vec<usize> = (0..prs_to_merge.len()).collect();
                                if let Ok(octocrab) = self.octocrab() {
                                    let repo = repo.clone();
                                    let prs = prs_to_merge;
                                    let dispatcher = dispatcher.clone();
                                    tokio::spawn(async move {
                                        let mut success = true;
                                        for &idx in &selected_indices {
                                            if let Some(pr) = prs.get(idx)
                                                && let Err(_) = crate::gh::merge(&octocrab, &repo, pr).await
                                            {
                                                success = false;
                                            }
                                        }
                                        let result = if success {
                                            Ok(())
                                        } else {
                                            Err("Some merges failed".to_string())
                                        };
                                        dispatcher.dispatch(Action::MergeComplete(result));
                                    });
                                }
                            }

                            // Enable auto-merge for building PRs
                            for pr in prs_to_auto_merge {
                                if let Ok(octocrab) = self.octocrab() {
                                    let repo = repo.clone();
                                    let pr_number = pr.number;
                                    let dispatcher = dispatcher.clone();
                                    tokio::spawn(async move {
                                        // Enable auto-merge on GitHub using GraphQL API
                                        let result = crate::task::enable_github_auto_merge(&octocrab, &repo, pr_number).await;

                                        match result {
                                            Ok(_) => {
                                                // Success - schedule periodic status checks
                                                dispatcher.dispatch(Action::SetTaskStatus(Some(crate::state::TaskStatus {
                                                    message: format!("Auto-merge enabled for PR #{}, monitoring...", pr_number),
                                                    status_type: crate::state::TaskStatusType::Success,
                                                })));

                                                // Spawn a task to periodically check PR status
                                                let dispatcher_clone = dispatcher.clone();
                                                let repo_clone = repo.clone();
                                                let octocrab_clone = octocrab.clone();
                                                tokio::spawn(async move {
                                                    for _ in 0..20 {
                                                        // Wait 1 minute between checks
                                                        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                                                        // Send status check result
                                                        dispatcher_clone
                                                            .dispatch(Action::AutoMergeStatusCheck(repo_index, pr_number));

                                                        // Check merge status to update PR state
                                                        if let Ok(pr_detail) = octocrab_clone
                                                            .pulls(&repo_clone.org, &repo_clone.repo)
                                                            .get(pr_number as u64)
                                                            .await
                                                        {
                                                            use crate::pr::MergeableStatus;

                                                            // Determine mergeable status
                                                            let mergeable_status = if pr_detail.merged_at.is_some() {
                                                                // PR has been merged - stop monitoring
                                                                let _ = dispatcher_clone.dispatch(
                                                                    Action::RemoveFromAutoMergeQueue(repo_index, pr_number),
                                                                );
                                                                let _ = dispatcher_clone.dispatch(Action::SetTaskStatus(Some(
                                                                    crate::state::TaskStatus {
                                                                        message: format!(
                                                                            "PR #{} successfully merged!",
                                                                            pr_number
                                                                        ),
                                                                        status_type: crate::state::TaskStatusType::Success,
                                                                    },
                                                                )));
                                                                break;
                                                            } else {
                                                                // Check CI status
                                                                match crate::task::get_pr_ci_status(
                                                                    &octocrab_clone,
                                                                    &repo_clone,
                                                                    &pr_detail.head.sha,
                                                                )
                                                                .await
                                                                {
                                                                    Ok((_, build_status)) => match build_status.as_str() {
                                                                        "success" | "neutral" | "skipped" => {
                                                                            MergeableStatus::Ready
                                                                        }
                                                                        "failure" | "cancelled" | "timed_out"
                                                                        | "action_required" => MergeableStatus::BuildFailed,
                                                                        _ => MergeableStatus::BuildInProgress,
                                                                    },
                                                                    Err(_) => MergeableStatus::Unknown,
                                                                }
                                                            };

                                                            // Update PR status
                                                            let _ = dispatcher_clone.dispatch(Action::MergeStatusUpdated(
                                                                repo_index,
                                                                pr_number,
                                                                mergeable_status,
                                                            ));
                                                        }
                                                    }
                                                });
                                            }
                                            Err(e) => {
                                                // Failed to enable auto-merge
                                                let _ = dispatcher
                                                    .dispatch(Action::RemoveFromAutoMergeQueue(repo_index, pr_number));
                                                dispatcher.dispatch(Action::SetTaskStatus(Some(crate::state::TaskStatus {
                                                    message: format!(
                                                        "Failed to enable auto-merge for PR #{}: {}",
                                                        pr_number, e
                                                    ),
                                                    status_type: crate::state::TaskStatusType::Error,
                                                })));
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }

                Action::Rebase => {
                    log::debug!("TaskMiddleware: Handling Rebase");

                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        // Check if there are selected PRs
                        let has_selection = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            !repo_data.selected_pr_numbers.is_empty()
                        } else {
                            false
                        };

                        // Get PRs to rebase
                        let prs_to_rebase: Vec<_> = if !has_selection {
                            // No selection - use current cursor PR
                            if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                if let Some(selected_idx) = repo_data.table_state.selected() {
                                    repo_data.prs.get(selected_idx).cloned().map(|pr| vec![pr]).unwrap_or_default()
                                } else {
                                    vec![]
                                }
                            } else {
                                vec![]
                            }
                        } else if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            repo_data
                                .prs
                                .iter()
                                .filter(|pr| {
                                    repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                })
                                .cloned()
                                .collect()
                        } else {
                            vec![]
                        };

                        if !prs_to_rebase.is_empty() {
                            // Start monitoring for each PR being rebased
                            for pr in &prs_to_rebase {
                                dispatcher.dispatch(Action::StartOperationMonitor(
                                    repo_index,
                                    pr.number,
                                    crate::state::OperationType::Rebase,
                                ));
                            }

                            // Set task status
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message: format!("Rebasing {} PR(s)...", prs_to_rebase.len()),
                                status_type: TaskStatusType::Running,
                            })));

                            // Spawn async task to rebase PRs
                            let selected_indices: Vec<usize> = (0..prs_to_rebase.len()).collect();
                            if let Ok(octocrab) = self.octocrab() {
                                let repo = repo.clone();
                                let prs = prs_to_rebase;
                                let dispatcher = dispatcher.clone();
                                tokio::spawn(async move {
                                    use crate::pr::MergeableStatus;

                                    let mut success = true;
                                    for &idx in &selected_indices {
                                        if let Some(pr) = prs.get(idx) {
                                            // For dependabot PRs, use comment-based rebase
                                            if pr.author.starts_with("dependabot") {
                                                // If PR has conflicts, use "@dependabot recreate" to rebuild the PR
                                                // Otherwise use "@dependabot rebase" for normal rebase
                                                let comment_text = if pr.mergeable == MergeableStatus::Conflicted {
                                                    "@dependabot recreate"
                                                } else {
                                                    "@dependabot rebase"
                                                };

                                                log::debug!(
                                                    "Posting comment '{}' to dependabot PR #{}",
                                                    comment_text, pr.number
                                                );
                                                match crate::gh::comment(&octocrab, &repo, pr, comment_text).await {
                                                    Ok(_) => {
                                                        log::debug!(
                                                            "Successfully posted comment to dependabot PR #{}",
                                                            pr.number
                                                        );
                                                    }
                                                    Err(e) => {
                                                        log::debug!(
                                                            "Failed to comment on dependabot PR #{}: {:?}",
                                                            pr.number, e
                                                        );
                                                        success = false;
                                                    }
                                                }
                                            } else {
                                                // For regular PRs, use GitHub's update_branch API
                                                // This performs a rebase/merge to bring the PR branch up to date with base
                                                log::debug!(
                                                    "Attempting to update branch for PR #{} in {}/{}",
                                                    pr.number, repo.org, repo.repo
                                                );
                                                let update_result = octocrab
                                                    .pulls(&repo.org, &repo.repo)
                                                    .update_branch(pr.number as u64)
                                                    .await;

                                                match update_result {
                                                    Ok(_) => {
                                                        log::debug!(
                                                            "Successfully triggered update_branch for PR #{}",
                                                            pr.number
                                                        );
                                                    }
                                                    Err(e) => {
                                                        log::debug!("Failed to update_branch for PR #{}: {:?}", pr.number, e);
                                                        success = false;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    let result = if success {
                                        Ok(())
                                    } else {
                                        Err("Some rebases failed".to_string())
                                    };
                                    dispatcher.dispatch(Action::RebaseComplete(result));
                                });
                            }
                        }
                    }
                }

                Action::ApprovePrs => {
                    log::debug!("TaskMiddleware: Handling ApprovePrs");

                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        let config = state.config.clone();

                        // Check if there are selected PRs
                        let has_selection = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            !repo_data.selected_pr_numbers.is_empty()
                        } else {
                            false
                        };

                        // Get PR numbers to approve
                        let pr_numbers: Vec<usize> = if !has_selection {
                            // No selection - use current cursor PR
                            if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                if let Some(selected_idx) = repo_data.table_state.selected() {
                                    repo_data
                                        .prs
                                        .get(selected_idx)
                                        .map(|pr| vec![pr.number])
                                        .unwrap_or_default()
                                } else {
                                    vec![]
                                }
                            } else {
                                vec![]
                            }
                        } else if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            repo_data
                                .prs
                                .iter()
                                .filter(|pr| {
                                    repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                })
                                .map(|pr| pr.number)
                                .collect()
                        } else {
                            vec![]
                        };

                        if !pr_numbers.is_empty() {
                            // Set task status
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message: format!("Approving {} PR(s)...", pr_numbers.len()),
                                status_type: TaskStatusType::Running,
                            })));

                            // Spawn async task to approve PRs
                            if let Ok(octocrab) = self.octocrab() {
                                let repo = repo.clone();
                                let approval_message = config.approval_message;
                                let dispatcher = dispatcher.clone();
                                tokio::spawn(async move {
                                    // Approve PRs using GitHub's review API
                                    let mut all_success = true;
                                    let mut approval_count = 0;

                                    for pr_number in &pr_numbers {
                                        // Create a review with APPROVE event using the REST API directly
                                        #[derive(serde::Serialize)]
                                        struct ReviewBody {
                                            body: String,
                                            event: String,
                                        }

                                        let review_body = ReviewBody {
                                            body: approval_message.clone(),
                                            event: "APPROVE".to_string(),
                                        };

                                        let url = format!(
                                            "/repos/{}/{}/pulls/{}/reviews",
                                            repo.org, repo.repo, pr_number
                                        );
                                        let result: Result<serde_json::Value, _> =
                                            octocrab.post(&url, Some(&review_body)).await;

                                        match result {
                                            Ok(_) => {
                                                approval_count += 1;
                                                log::debug!("Successfully approved PR #{}", pr_number);
                                            }
                                            Err(e) => {
                                                all_success = false;
                                                log::debug!("Failed to approve PR #{}: {}", pr_number, e);
                                            }
                                        }
                                    }

                                    let result = if all_success && approval_count > 0 {
                                        Ok(())
                                    } else if approval_count == 0 {
                                        Err("Failed to approve any PRs".to_string())
                                    } else {
                                        Err(format!(
                                            "Approved {}/{} PRs",
                                            approval_count,
                                            pr_numbers.len()
                                        ))
                                    };
                                    dispatcher.dispatch(Action::ApprovalComplete(result));
                                });
                            }
                        }
                    }
                }

                Action::ClosePrFormSubmit => {
                    log::debug!("TaskMiddleware: Handling ClosePrFormSubmit");

                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        // Get comment from close_pr_state
                        if let Some(close_pr) = &state.ui.close_pr_state {
                            let comment = close_pr.comment.clone();

                            // Check if there are selected PRs
                            let has_selection = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                !repo_data.selected_pr_numbers.is_empty()
                            } else {
                                false
                            };

                            // Get PR numbers and PRs to close
                            let (pr_numbers, prs): (Vec<usize>, Vec<crate::pr::Pr>) = if !has_selection {
                                // No selection - use current cursor PR
                                if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                    if let Some(selected_idx) = repo_data.table_state.selected() {
                                        repo_data
                                            .prs
                                            .get(selected_idx)
                                            .cloned()
                                            .map(|pr| (vec![pr.number], vec![pr]))
                                            .unwrap_or((vec![], vec![]))
                                    } else {
                                        (vec![], vec![])
                                    }
                                } else {
                                    (vec![], vec![])
                                }
                            } else if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                let selected_prs: Vec<_> = repo_data
                                    .prs
                                    .iter()
                                    .filter(|pr| {
                                        repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                    })
                                    .cloned()
                                    .collect();
                                let pr_nums: Vec<usize> = selected_prs.iter().map(|pr| pr.number).collect();
                                (pr_nums, selected_prs)
                            } else {
                                (vec![], vec![])
                            };

                            if !pr_numbers.is_empty() {
                                // Set task status
                                dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                    message: format!("Closing {} PR(s)...", pr_numbers.len()),
                                    status_type: TaskStatusType::Running,
                                })));

                                // Spawn async task to close PRs
                                if let Ok(octocrab) = self.octocrab() {
                                    let repo = repo.clone();
                                    let dispatcher = dispatcher.clone();
                                    tokio::spawn(async move {
                                        // Close PRs with comment (use @dependabot close for dependabot PRs)
                                        let mut all_success = true;
                                        let mut close_count = 0;

                                        for pr_number in &pr_numbers {
                                            // Find the full PR object to check author
                                            let pr = prs.iter().find(|p| p.number == *pr_number);
                                            let is_dependabot = pr
                                                .map(|p| p.author.to_lowercase().contains("dependabot"))
                                                .unwrap_or(false);

                                            let actual_comment = if is_dependabot {
                                                "@dependabot close".to_string()
                                            } else {
                                                comment.clone()
                                            };

                                            // First, add a comment using octocrab issues API
                                            if let Err(e) = octocrab
                                                .issues(&repo.org, &repo.repo)
                                                .create_comment(*pr_number as _, &actual_comment)
                                                .await
                                            {
                                                log::debug!("Failed to add comment to PR #{}: {}", pr_number, e);
                                                all_success = false;
                                                continue;
                                            }

                                            // For dependabot PRs, just the comment is enough
                                            if is_dependabot {
                                                close_count += 1;
                                                log::debug!("Added '@dependabot close' comment to PR #{}", pr_number);
                                            } else {
                                                // For regular PRs, close the PR via API
                                                #[derive(serde::Serialize)]
                                                struct UpdatePrBody {
                                                    state: String,
                                                }

                                                let update_body = UpdatePrBody {
                                                    state: "closed".to_string(),
                                                };

                                                let url = format!("/repos/{}/{}/pulls/{}", repo.org, repo.repo, pr_number);
                                                let result: Result<serde_json::Value, _> =
                                                    octocrab.patch(&url, Some(&update_body)).await;

                                                match result {
                                                    Ok(_) => {
                                                        close_count += 1;
                                                        log::debug!("Successfully closed PR #{}", pr_number);
                                                    }
                                                    Err(e) => {
                                                        all_success = false;
                                                        log::debug!("Failed to close PR #{}: {}", pr_number, e);
                                                    }
                                                }
                                            }
                                        }

                                        let result = if all_success && close_count > 0 {
                                            Ok(())
                                        } else if close_count == 0 {
                                            Err("Failed to close any PRs".to_string())
                                        } else {
                                            Err(format!("Closed {}/{} PRs", close_count, pr_numbers.len()))
                                        };
                                        dispatcher.dispatch(Action::ClosePrComplete(result));
                                    });
                                }
                            }
                        }
                    }
                }

                //
                // BACKGROUND CHECKS & MONITORING
                //

                Action::StartOperationMonitor(repo_index, pr_number, operation) => {
                    log::debug!("TaskMiddleware: Handling StartOperationMonitor for PR #{}", pr_number);

                    if let Some(repo) = state.repos.recent_repos.get(*repo_index).cloned() {
                        // Spawn async task for operation monitoring
                        if let Ok(octocrab) = self.octocrab() {
                            let repo_index = *repo_index;
                            let pr_number = *pr_number;
                            let operation = *operation;
                            let dispatcher_clone = dispatcher.clone();
                            let repo_clone = repo.clone();
                            let octocrab_clone = octocrab.clone();

                            tokio::spawn(async move {
                                use crate::pr::MergeableStatus;
                                use crate::state::OperationType;

                                log::debug!(
                                    "Starting operation monitor for PR #{} ({:?})",
                                    pr_number, operation
                                );

                                // Get initial PR state to track SHA for rebase detection
                                let mut last_head_sha = None;
                                if let Ok(pr_detail) = octocrab_clone
                                    .pulls(&repo_clone.org, &repo_clone.repo)
                                    .get(pr_number as u64)
                                    .await
                                {
                                    last_head_sha = Some(pr_detail.head.sha.clone());
                                    log::debug!("Initial SHA for PR #{}: {}", pr_number, pr_detail.head.sha);
                                }

                                // Track consecutive failures to avoid infinite loops
                                let mut consecutive_failures = 0;
                                const MAX_CONSECUTIVE_FAILURES: u32 = 5;

                                // Monitor for up to 120 checks (1 hour at 30s intervals)
                                for check_num in 0..120 {
                                    // Wait between checks (30 seconds)
                                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

                                    log::debug!(
                                        "Operation monitor check #{} for PR #{}",
                                        check_num + 1,
                                        pr_number
                                    );

                                    // Send periodic check action
                                    dispatcher_clone.dispatch(Action::OperationMonitorCheck(repo_index, pr_number));

                                    // Fetch current PR state
                                    let pr_detail = match octocrab_clone
                                        .pulls(&repo_clone.org, &repo_clone.repo)
                                        .get(pr_number as u64)
                                        .await
                                    {
                                        Ok(pr) => {
                                            consecutive_failures = 0; // Reset on success
                                            pr
                                        }
                                        Err(e) => {
                                            consecutive_failures += 1;
                                            log::debug!(
                                                "Failed to fetch PR #{} (attempt {}/{}): {}",
                                                pr_number, consecutive_failures, MAX_CONSECUTIVE_FAILURES, e
                                            );

                                            if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                                                log::debug!(
                                                    "Too many consecutive failures for PR #{}, stopping monitor",
                                                    pr_number
                                                );
                                                let _ = dispatcher_clone.dispatch(
                                                    Action::RemoveFromOperationMonitor(repo_index, pr_number),
                                                );
                                                let _ = dispatcher_clone.dispatch(Action::SetTaskStatus(Some(
                                                    crate::state::TaskStatus {
                                                        message: format!(
                                                            "Monitoring stopped for PR #{} due to API errors",
                                                            pr_number
                                                        ),
                                                        status_type: crate::state::TaskStatusType::Error,
                                                    },
                                                )));
                                                break;
                                            }
                                            continue; // Skip this check if API fails
                                        }
                                    };

                                    match operation {
                                        OperationType::Rebase => {
                                            // Check if head SHA changed (rebase completed)
                                            let current_sha = pr_detail.head.sha.clone();
                                            let sha_changed = if let Some(ref prev_sha) = last_head_sha {
                                                if &current_sha != prev_sha {
                                                    log::debug!(
                                                        "PR #{} SHA changed: {} -> {}",
                                                        pr_number, prev_sha, current_sha
                                                    );
                                                    true
                                                } else {
                                                    false
                                                }
                                            } else {
                                                log::debug!("PR #{} first check, SHA: {}", pr_number, current_sha);
                                                false
                                            };

                                            // Update last SHA
                                            last_head_sha = Some(current_sha.clone());

                                            // Check CI status (always check after initial rebasing time)
                                            if sha_changed || check_num > 2 {
                                                log::debug!(
                                                    "Checking CI status for PR #{} at SHA {}",
                                                    pr_number, current_sha
                                                );

                                                match crate::task::get_pr_ci_status(&octocrab_clone, &repo_clone, &current_sha)
                                                    .await
                                                {
                                                    Ok((_, build_status)) => {
                                                        log::debug!("PR #{} CI status: {}", pr_number, build_status);

                                                        let new_status = match build_status.as_str() {
                                                            "success" | "neutral" | "skipped" => {
                                                                MergeableStatus::Ready
                                                            }
                                                            "failure" | "cancelled" | "timed_out"
                                                            | "action_required" => MergeableStatus::BuildFailed,
                                                            "pending" | "in_progress" | "queued" => {
                                                                MergeableStatus::BuildInProgress
                                                            }
                                                            "unknown" => {
                                                                // No CI configured - treat as ready after rebase completes
                                                                if sha_changed {
                                                                    log::debug!(
                                                                        "No CI found for PR #{}, treating as ready",
                                                                        pr_number
                                                                    );
                                                                    MergeableStatus::Ready
                                                                } else {
                                                                    MergeableStatus::Rebasing
                                                                }
                                                            }
                                                            _ => {
                                                                log::debug!(
                                                                    "Unknown CI status '{}' for PR #{}, treating as in progress",
                                                                    build_status, pr_number
                                                                );
                                                                MergeableStatus::BuildInProgress
                                                            }
                                                        };

                                                        // Update status
                                                        let _ =
                                                            dispatcher_clone.dispatch(Action::MergeStatusUpdated(
                                                                repo_index, pr_number, new_status,
                                                            ));

                                                        // If CI is done (or no CI), stop monitoring
                                                        if matches!(
                                                            new_status,
                                                            MergeableStatus::Ready | MergeableStatus::BuildFailed
                                                        ) {
                                                            log::debug!(
                                                                "PR #{} monitoring complete with status {:?}",
                                                                pr_number, new_status
                                                            );
                                                            let _ = dispatcher_clone.dispatch(
                                                                Action::RemoveFromOperationMonitor(
                                                                    repo_index, pr_number,
                                                                ),
                                                            );
                                                            break;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        consecutive_failures += 1;
                                                        log::debug!(
                                                            "Failed to get CI status for PR #{} (attempt {}/{}): {}",
                                                            pr_number,
                                                            consecutive_failures,
                                                            MAX_CONSECUTIVE_FAILURES,
                                                            e
                                                        );

                                                        if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                                                            log::debug!(
                                                                "Too many CI status failures for PR #{}, stopping monitor",
                                                                pr_number
                                                            );
                                                            let _ = dispatcher_clone.dispatch(
                                                                Action::RemoveFromOperationMonitor(
                                                                    repo_index, pr_number,
                                                                ),
                                                            );
                                                            let _ = dispatcher_clone.dispatch(
                                                                Action::MergeStatusUpdated(
                                                                    repo_index,
                                                                    pr_number,
                                                                    MergeableStatus::Unknown,
                                                                ),
                                                            );
                                                            break;
                                                        }

                                                        // Set to building while we retry
                                                        let _ =
                                                            dispatcher_clone.dispatch(Action::MergeStatusUpdated(
                                                                repo_index,
                                                                pr_number,
                                                                MergeableStatus::BuildInProgress,
                                                            ));
                                                    }
                                                }
                                            }
                                        }
                                        OperationType::Merge => {
                                            // Check if PR is merged
                                            if pr_detail.merged_at.is_some() {
                                                // Merge successful!
                                                log::debug!("PR #{} successfully merged!", pr_number);
                                                let _ = dispatcher_clone.dispatch(
                                                    Action::RemoveFromOperationMonitor(repo_index, pr_number),
                                                );
                                                let _ = dispatcher_clone.dispatch(Action::SetTaskStatus(Some(
                                                    crate::state::TaskStatus {
                                                        message: format!("PR #{} successfully merged!", pr_number),
                                                        status_type: crate::state::TaskStatusType::Success,
                                                    },
                                                )));
                                                // Trigger repo reload to remove merged PR from list
                                                let _ = dispatcher_clone.dispatch(Action::ReloadRepo(repo_index));
                                                break;
                                            } else if matches!(
                                                pr_detail.state,
                                                Some(octocrab::models::IssueState::Closed)
                                            ) {
                                                // PR was closed without merging
                                                log::debug!("PR #{} was closed without merging", pr_number);
                                                let _ = dispatcher_clone.dispatch(
                                                    Action::RemoveFromOperationMonitor(repo_index, pr_number),
                                                );
                                                let _ = dispatcher_clone.dispatch(Action::SetTaskStatus(Some(
                                                    crate::state::TaskStatus {
                                                        message: format!(
                                                            "PR #{} was closed without merging",
                                                            pr_number
                                                        ),
                                                        status_type: crate::state::TaskStatusType::Error,
                                                    },
                                                )));
                                                break;
                                            }

                                            // Update status to show we're still merging
                                            log::debug!("PR #{} still merging (check #{})", pr_number, check_num + 1);
                                            let _ = dispatcher_clone.dispatch(Action::MergeStatusUpdated(
                                                repo_index,
                                                pr_number,
                                                MergeableStatus::Merging,
                                            ));
                                        }
                                    }
                                }

                                // If we exit the loop without completing, it's a timeout
                                log::debug!(
                                    "Operation monitor timed out for PR #{} after 1 hour",
                                    pr_number
                                );
                                let _ = dispatcher_clone
                                    .dispatch(Action::RemoveFromOperationMonitor(repo_index, pr_number));
                                let _ = dispatcher_clone.dispatch(Action::SetTaskStatus(Some(
                                    crate::state::TaskStatus {
                                        message: format!("Monitoring timed out for PR #{} after 1 hour", pr_number),
                                        status_type: crate::state::TaskStatusType::Warning,
                                    },
                                )));
                            });
                        }
                    }
                }

                Action::RerunFailedJobs => {
                    log::debug!("TaskMiddleware: Handling RerunFailedJobs");

                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        // Check if there are selected PRs
                        let has_selection = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            !repo_data.selected_pr_numbers.is_empty()
                        } else {
                            false
                        };

                        // Get PR numbers to rerun
                        let pr_numbers: Vec<usize> = if !has_selection {
                            // No selection - use current cursor PR
                            if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                if let Some(selected_idx) = repo_data.table_state.selected() {
                                    repo_data
                                        .prs
                                        .get(selected_idx)
                                        .map(|pr| vec![pr.number])
                                        .unwrap_or_default()
                                } else {
                                    vec![]
                                }
                            } else {
                                vec![]
                            }
                        } else if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            repo_data
                                .prs
                                .iter()
                                .filter(|pr| {
                                    repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                })
                                .map(|pr| pr.number)
                                .collect()
                        } else {
                            vec![]
                        };

                        if !pr_numbers.is_empty() {
                            // Spawn async task to rerun failed jobs
                            if let Ok(octocrab) = self.octocrab() {
                                let repo = repo.clone();
                                let dispatcher = dispatcher.clone();
                                tokio::spawn(async move {
                                    let mut all_success = true;
                                    let mut rerun_count = 0;

                                    for pr_number in pr_numbers {
                                        // Get PR details to find head SHA
                                        let pr = match octocrab
                                            .pulls(&repo.org, &repo.repo)
                                            .get(pr_number as u64)
                                            .await
                                        {
                                            Ok(pr) => pr,
                                            Err(_) => {
                                                all_success = false;
                                                continue;
                                            }
                                        };

                                        let head_sha = &pr.head.sha;

                                        // Get workflow runs for this PR using REST API
                                        let url = format!(
                                            "/repos/{}/{}/actions/runs?head_sha={}",
                                            repo.org, repo.repo, head_sha
                                        );

                                        #[derive(Debug, serde::Deserialize)]
                                        struct WorkflowRunsResponse {
                                            workflow_runs: Vec<octocrab::models::workflows::Run>,
                                        }

                                        let workflow_response: WorkflowRunsResponse =
                                            match octocrab.get(&url, None::<&()>).await {
                                                Ok(response) => response,
                                                Err(_) => {
                                                    all_success = false;
                                                    continue;
                                                }
                                            };

                                        let runs = workflow_response.workflow_runs;

                                        // Find failed runs and rerun them
                                        for run in runs {
                                            let is_failed = run.conclusion.as_deref() == Some("failure");
                                            if is_failed {
                                                // Rerun failed jobs for this run
                                                let url = format!(
                                                    "https://api.github.com/repos/{}/{}/actions/runs/{}/rerun-failed-jobs",
                                                    repo.org, repo.repo, run.id
                                                );

                                                // Use serde_json::Value as response type for POST requests
                                                match octocrab
                                                    .post::<(), serde_json::Value>(&url, None::<&()>)
                                                    .await
                                                {
                                                    Ok(_) => {
                                                        rerun_count += 1;
                                                    }
                                                    Err(_) => {
                                                        all_success = false;
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    let result = if all_success && rerun_count > 0 {
                                        Ok(())
                                    } else if rerun_count == 0 {
                                        Err("No failed jobs found to rerun".to_string())
                                    } else {
                                        Err("Some jobs failed to rerun".to_string())
                                    };
                                    dispatcher.dispatch(Action::RerunJobsComplete(result));
                                });
                            }
                        }
                    }
                }

                Action::OpenBuildLogs => {
                    log::debug!("TaskMiddleware: Handling OpenBuildLogs");

                    let repo_index = state.repos.selected_repo;
                    if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                        if let Some(selected_idx) = repo_data.table_state.selected() {
                            if let Some(pr) = repo_data.prs.get(selected_idx).cloned() {
                                if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                                    // Set status
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: "Loading build logs...".to_string(),
                                        status_type: TaskStatusType::Running,
                                    })));

                                    // Spawn async task to fetch build logs
                                    if let Ok(octocrab) = self.octocrab() {
                                        let pr_context = crate::log::PrContext {
                                            number: pr.number,
                                            title: pr.title.clone(),
                                            author: pr.author.clone(),
                                        };
                                        let pr_number = pr.number;
                                        let repo = repo.clone();
                                        let dispatcher = dispatcher.clone();
                                        tokio::spawn(async move {
                                            // First, get the PR details to get the actual head SHA
                                            let pr_details = match octocrab
                                                .pulls(&repo.org, &repo.repo)
                                                .get(pr_number as u64)
                                                .await
                                            {
                                                Ok(pr) => pr,
                                                Err(_) => {
                                                    dispatcher.dispatch(Action::BuildLogsLoaded(vec![], pr_context));
                                                    return;
                                                }
                                            };

                                            let head_sha = pr_details.head.sha.clone();

                                            // Get workflow runs for this commit using the REST API directly
                                            let url = format!(
                                                "/repos/{}/{}/actions/runs?head_sha={}",
                                                repo.org, repo.repo, head_sha
                                            );

                                            #[derive(Debug, serde::Deserialize)]
                                            struct WorkflowRunsResponse {
                                                workflow_runs: Vec<octocrab::models::workflows::Run>,
                                            }

                                            let workflow_runs: WorkflowRunsResponse = match octocrab.get(&url, None::<&()>).await {
                                                Ok(runs) => runs,
                                                Err(_) => {
                                                    dispatcher.dispatch(Action::BuildLogsLoaded(vec![], pr_context));
                                                    return;
                                                }
                                            };

                                            let mut log_sections = Vec::new();

                                            // Process each workflow run and download its logs
                                            for workflow_run in workflow_runs.workflow_runs {
                                                let conclusion_str = workflow_run.conclusion.as_deref().unwrap_or("in_progress");
                                                let workflow_name = workflow_run.name.clone();

                                                // Skip successful runs unless there are no failures
                                                let is_failed = matches!(
                                                    conclusion_str,
                                                    "failure" | "timed_out" | "action_required" | "cancelled"
                                                );

                                                if !is_failed && !log_sections.is_empty() {
                                                    continue;
                                                }

                                                // Fetch jobs for this workflow run to get job IDs and URLs
                                                let jobs_url = format!(
                                                    "/repos/{}/{}/actions/runs/{}/jobs",
                                                    repo.org, repo.repo, workflow_run.id
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
                                                    started_at: String,
                                                    completed_at: Option<String>,
                                                }

                                                let jobs_response: Result<JobsResponse, _> =
                                                    octocrab.get(&jobs_url, None::<&()>).await;

                                                // Try to download the workflow run logs (they come as a zip file)
                                                match octocrab
                                                    .actions()
                                                    .download_workflow_run_logs(&repo.org, &repo.repo, workflow_run.id)
                                                    .await
                                                {
                                                    Ok(log_data) => {
                                                        // The log_data is a zip file as bytes
                                                        // Parse using the gh-actions-log-parser crate
                                                        match gh_actions_log_parser::parse_workflow_logs(&log_data) {
                                                            Ok(parsed_log) => {
                                                                // Process each job's logs and build metadata
                                                                for job_log in parsed_log.jobs {
                                                                    // Try to find matching GitHub API job by name
                                                                    let github_job = if let Ok(ref jobs) = jobs_response {
                                                                        jobs.jobs.iter().find(|j| job_log.name.contains(&j.name))
                                                                    } else {
                                                                        None
                                                                    };

                                                                    // Extract real job name from log content (look for "Complete job name:" line)
                                                                    let mut display_name = job_log.name.clone();
                                                                    for line in &job_log.lines {
                                                                        if line.content.contains("Complete job name:") {
                                                                            // Extract: "2025-11-15T19:56:48.3220210Z Complete job name: lint (macos-latest, clippy)"
                                                                            if let Some(name_part) =
                                                                                line.content.split("Complete job name:").nth(1)
                                                                            {
                                                                                display_name = name_part.trim().to_string();
                                                                                break;
                                                                            }
                                                                        }
                                                                    }

                                                                    // Count errors in this job
                                                                    let error_count = job_log.lines.iter().filter(|line| {
                                                                        // Count lines with error workflow command OR containing "error:"
                                                                        if let Some(ref cmd) = line.command {
                                                                            matches!(cmd, gh_actions_log_parser::WorkflowCommand::Error { .. })
                                                                        } else {
                                                                            line.content.to_lowercase().contains("error:")
                                                                        }
                                                                    }).count();

                                                                    // Parse job status from GitHub API
                                                                    let status = if let Some(job) = github_job {
                                                                        match job.conclusion.as_deref() {
                                                                            Some("success") => crate::log::JobStatus::Success,
                                                                            Some("failure") => crate::log::JobStatus::Failure,
                                                                            Some("cancelled") => crate::log::JobStatus::Cancelled,
                                                                            Some("skipped") => crate::log::JobStatus::Skipped,
                                                                            None => crate::log::JobStatus::InProgress,
                                                                            _ => crate::log::JobStatus::Unknown,
                                                                        }
                                                                    } else {
                                                                        // Infer from error count if no API data
                                                                        if error_count > 0 {
                                                                            crate::log::JobStatus::Failure
                                                                        } else {
                                                                            crate::log::JobStatus::Success
                                                                        }
                                                                    };

                                                                    // Calculate duration from GitHub API
                                                                    let duration = if let Some(job) = github_job {
                                                                        if let Some(ref completed) = job.completed_at {
                                                                            // Parse timestamps and calculate duration
                                                                            use chrono::DateTime;
                                                                            if let (Ok(started), Ok(completed)) = (
                                                                                DateTime::parse_from_rfc3339(&job.started_at),
                                                                                DateTime::parse_from_rfc3339(completed),
                                                                            ) {
                                                                                let duration =
                                                                                    completed.signed_duration_since(started);
                                                                                Some(std::time::Duration::from_secs(
                                                                                    duration.num_seconds().max(0) as u64,
                                                                                ))
                                                                            } else {
                                                                                None
                                                                            }
                                                                        } else {
                                                                            None
                                                                        }
                                                                    } else {
                                                                        None
                                                                    };

                                                                    // Build job metadata
                                                                    let metadata = crate::log::JobMetadata {
                                                                        name: display_name,
                                                                        workflow_name: workflow_name.clone(),
                                                                        status,
                                                                        error_count,
                                                                        duration,
                                                                        html_url: github_job
                                                                            .map(|j| j.html_url.clone())
                                                                            .unwrap_or_default(),
                                                                    };

                                                                    // Add to jobs list (preserve full JobLog from parser)
                                                                    log_sections.push((metadata, job_log));
                                                                }
                                                            }
                                                            Err(_err) => {
                                                                // Parser error - skip this workflow run
                                                                // User will see error in the PR list or can open in browser
                                                            }
                                                        }
                                                    }
                                                    Err(_) => {
                                                        // Download error - skip this workflow run
                                                        // Common if logs expired or auth issues
                                                    }
                                                }
                                            }

                                            // Sort jobs: failed first, then successful
                                            // UI will display them in order with failed at top
                                            log_sections.sort_by_key(|(metadata, _)| match metadata.status {
                                                crate::log::JobStatus::Failure => 0,
                                                crate::log::JobStatus::Cancelled => 1,
                                                crate::log::JobStatus::InProgress => 2,
                                                crate::log::JobStatus::Unknown => 3,
                                                crate::log::JobStatus::Skipped => 4,
                                                crate::log::JobStatus::Success => 5,
                                            });

                                            dispatcher.dispatch(Action::BuildLogsLoaded(log_sections, pr_context));
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                Action::StartMergeBot => {
                    log::debug!("TaskMiddleware: Handling StartMergeBot");

                    let repo_index = state.repos.selected_repo;
                    if let Some(_repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        // Get selected PRs
                        let prs_to_process: Vec<_> = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            repo_data
                                .prs
                                .iter()
                                .filter(|pr| {
                                    repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                })
                                .cloned()
                                .collect()
                        } else {
                            vec![]
                        };

                        if !prs_to_process.is_empty() {
                            // Build PR data for merge bot initialization
                            let pr_data: Vec<(usize, usize)> = prs_to_process
                                .iter()
                                .enumerate()
                                .map(|(idx, pr)| (pr.number, idx))
                                .collect();

                            // Dispatch action to initialize bot
                            dispatcher.dispatch(Action::StartMergeBotWithPrData(pr_data));
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message: format!("Merge bot started with {} PR(s)", prs_to_process.len()),
                                status_type: TaskStatusType::Success,
                            })));
                        }
                    }
                }

                Action::StartRecurringUpdates(interval_ms) => {
                    log::debug!("TaskMiddleware: Handling StartRecurringUpdates");

                    // Spawn recurring task directly
                    let dispatcher_clone = dispatcher.clone();
                    let interval_ms = *interval_ms;
                    tokio::spawn(async move {
                        log::debug!(
                            "Starting recurring task with interval: {}ms ({} minutes)",
                            interval_ms,
                            interval_ms / 60000
                        );
                        loop {
                            // Sleep for the interval
                            tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;

                            log::debug!(
                                "Recurring task triggered (interval: {}ms), dispatching RecurringUpdateTriggered",
                                interval_ms
                            );

                            // Dispatch the configured action
                            let _ = dispatcher_clone.dispatch(Action::RecurringUpdateTriggered);
                        }
                    });
                }

                //
                // CACHE MANAGEMENT
                //

                Action::ClearCache => {
                    log::debug!("TaskMiddleware: Handling ClearCache");

                    // Clear the cache
                    if let Ok(mut cache) = self.cache.lock() {
                        if cache.clear().is_ok() {
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message: "Cache cleared".to_string(),
                                status_type: TaskStatusType::Success,
                            })));
                        } else {
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message: "Failed to clear cache".to_string(),
                                status_type: TaskStatusType::Error,
                            })));
                        }
                    }
                }

                Action::ShowCacheStats => {
                    log::debug!("TaskMiddleware: Handling ShowCacheStats");

                    // Show cache statistics
                    if let Ok(cache) = self.cache.lock() {
                        let stats = cache.stats();
                        dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                            message: format!(
                                "Cache: {} total, {} fresh, {} stale (TTL: {}s)",
                                stats.total_entries, stats.fresh_entries, stats.stale_entries, stats.ttl_seconds
                            ),
                            status_type: TaskStatusType::Success,
                        })));
                    }
                }

                Action::InvalidateRepoCache(repo_index) => {
                    log::debug!("TaskMiddleware: Handling InvalidateRepoCache for repo {}", repo_index);

                    // Invalidate cache for specific repo using pattern matching
                    if let Some(repo) = state.repos.recent_repos.get(*repo_index) {
                        if let Ok(mut cache) = self.cache.lock() {
                            let pattern = format!("{}/{}", repo.org, repo.repo);
                            cache.invalidate_pattern(&pattern);
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message: format!("Cache invalidated for {}/{}", repo.org, repo.repo),
                                status_type: TaskStatusType::Success,
                            })));
                        }
                    }
                }

                // All other actions pass through unchanged
                _ => {}
            }

            // Always continue to next middleware/reducer
            true
        })
    }
}
