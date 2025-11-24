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
    /// Task channel for background operations (legacy - used during migration)
    task_tx: tokio::sync::mpsc::UnboundedSender<crate::task::BackgroundTask>,
}

impl TaskMiddleware {
    pub fn new(
        cache: std::sync::Arc<std::sync::Mutex<gh_api_cache::ApiCache>>,
        task_tx: tokio::sync::mpsc::UnboundedSender<crate::task::BackgroundTask>,
    ) -> Self {
        Self {
            octocrab: None,
            cache,
            task_tx,
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
            use crate::task::BackgroundTask;

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

                        // Trigger background task (using legacy system for now)
                        if let Ok(octocrab) = self.octocrab() {
                            let _ = self.task_tx.send(BackgroundTask::LoadSingleRepo {
                                repo_index,
                                repo,
                                filter,
                                octocrab,
                                cache: self.cache.clone(),
                                bypass_cache: true, // Refresh always bypasses cache
                                dispatcher: dispatcher.clone(),
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

                        // Trigger background task
                        if let Ok(octocrab) = self.octocrab() {
                            let _ = self.task_tx.send(BackgroundTask::LoadSingleRepo {
                                repo_index: *repo_index,
                                repo,
                                filter,
                                octocrab,
                                cache: self.cache.clone(),
                                bypass_cache: false, // Normal reload uses cache
                                dispatcher: dispatcher.clone(),
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

                            // Send background task to open in IDE
                            let _ = self.task_tx.send(BackgroundTask::OpenPRInIDE {
                                repo,
                                pr_number,
                                ide_command: config.ide_command,
                                temp_dir: config.temp_dir,
                                dispatcher: dispatcher.clone(),
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

                                // Send background task
                                let selected_indices: Vec<usize> = (0..prs_to_merge.len()).collect();
                                if let Ok(octocrab) = self.octocrab() {
                                    let _ = self.task_tx.send(BackgroundTask::Merge {
                                        repo: repo.clone(),
                                        prs: prs_to_merge,
                                        selected_indices,
                                        octocrab,
                                dispatcher: dispatcher.clone(),
                            });
                                }
                            }

                            // Enable auto-merge for building PRs (still using legacy system)
                            for pr in prs_to_auto_merge {
                                if let Ok(octocrab) = self.octocrab() {
                                    let _ = self.task_tx.send(BackgroundTask::EnableAutoMerge {
                                        repo_index,
                                        repo: repo.clone(),
                                        pr_number: pr.number,
                                        octocrab,
                                dispatcher: dispatcher.clone(),
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

                            // Send background task
                            let selected_indices: Vec<usize> = (0..prs_to_rebase.len()).collect();
                            if let Ok(octocrab) = self.octocrab() {
                                let _ = self.task_tx.send(BackgroundTask::Rebase {
                                    repo,
                                    prs: prs_to_rebase,
                                    selected_indices,
                                    octocrab,
                                dispatcher: dispatcher.clone(),
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

                            // Send background task
                            if let Ok(octocrab) = self.octocrab() {
                                let _ = self.task_tx.send(BackgroundTask::ApprovePrs {
                                    repo,
                                    pr_numbers,
                                    approval_message: config.approval_message,
                                    octocrab,
                                dispatcher: dispatcher.clone(),
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

                                // Send background task
                                if let Ok(octocrab) = self.octocrab() {
                                    let _ = self.task_tx.send(BackgroundTask::ClosePrs {
                                        repo,
                                        pr_numbers: pr_numbers.clone(),
                                        prs,
                                        comment,
                                        octocrab,
                                dispatcher: dispatcher.clone(),
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
                        // Send background task for operation monitoring
                        if let Ok(octocrab) = self.octocrab() {
                            let _ = self.task_tx.send(BackgroundTask::MonitorOperation {
                                repo_index: *repo_index,
                                repo,
                                pr_number: *pr_number,
                                operation: *operation,
                                octocrab,
                                dispatcher: dispatcher.clone(),
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
                            // Send background task
                            if let Ok(octocrab) = self.octocrab() {
                                let _ = self.task_tx.send(BackgroundTask::RerunFailedJobs {
                                    repo,
                                    pr_numbers,
                                    octocrab,
                                dispatcher: dispatcher.clone(),
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

                                    // Send background task to fetch build logs
                                    if let Ok(octocrab) = self.octocrab() {
                                        let pr_context = crate::log::PrContext {
                                            number: pr.number,
                                            title: pr.title.clone(),
                                            author: pr.author.clone(),
                                        };
                                        let _ = self.task_tx.send(BackgroundTask::FetchBuildLogs {
                                            repo,
                                            pr_number: pr.number,
                                            head_sha: "HEAD".to_string(), // Placeholder - will fetch in background task
                                            octocrab,
                                            pr_context,
                                            dispatcher: dispatcher.clone(),
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

                    // Send background task to start recurring updates
                    let _ = self.task_tx.send(BackgroundTask::RecurringTask {
                        action: Action::RecurringUpdateTriggered,
                        interval_ms: *interval_ms,
                        dispatcher: dispatcher.clone(),
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
