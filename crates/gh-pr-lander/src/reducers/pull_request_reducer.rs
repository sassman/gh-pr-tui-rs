//! PR Reducer
//!
//! Handles state updates for Pull Request data using tagged PullRequestAction.

use crate::actions::PullRequestAction;
use crate::domain_models::{LoadingState, Repository};
use crate::state::MainViewState;

/// Find repository index by Repository
fn find_repo_idx(state: &MainViewState, repo: &Repository) -> Option<usize> {
    state
        .repositories
        .iter()
        .position(|r| r.org == repo.org && r.repo == repo.repo)
}

/// Reduce PR-related state based on actions (new tagged action version)
///
/// Accepts only PullRequestAction, making it type-safe and focused.
pub fn reduce_pull_request(mut state: MainViewState, action: &PullRequestAction) -> MainViewState {
    match action {
        PullRequestAction::LoadStart { repo } => {
            // Find repo index
            let Some(repo_idx) = find_repo_idx(&state, repo) else {
                log::warn!(
                    "LoadStart: Repository {}/{} not found in state",
                    repo.org,
                    repo.repo
                );
                return state;
            };
            // Set loading state for the repository
            let repo_data = state.repo_data.entry(repo_idx).or_default();
            repo_data.loading_state = LoadingState::Loading;
            log::debug!(
                "PR loading started for repository {}/{}",
                repo.org,
                repo.repo
            );
        }

        PullRequestAction::Loaded { repo, prs } => {
            // Find repo index
            let Some(repo_idx) = find_repo_idx(&state, repo) else {
                log::warn!(
                    "Loaded: Repository {}/{} not found in state",
                    repo.org,
                    repo.repo
                );
                return state;
            };
            // Update repository data with loaded PRs
            let repo_data = state.repo_data.entry(repo_idx).or_default();
            repo_data.prs = prs.clone();
            repo_data.loading_state = LoadingState::Loaded;
            repo_data.last_updated = Some(chrono::Local::now());
            repo_data.selected_pr = 0;
            // Clear selection when PRs are reloaded
            repo_data.selected_pr_numbers.clear();
            log::info!(
                "Loaded {} PRs for repository {}/{}",
                prs.len(),
                repo.org,
                repo.repo
            );
        }

        PullRequestAction::LoadError { repo, error } => {
            // Find repo index
            let Some(repo_idx) = find_repo_idx(&state, repo) else {
                log::warn!(
                    "LoadError: Repository {}/{} not found in state",
                    repo.org,
                    repo.repo
                );
                return state;
            };
            // Set error state for the repository
            let repo_data = state.repo_data.entry(repo_idx).or_default();
            repo_data.loading_state = LoadingState::Error(error.clone());
            log::error!(
                "Failed to load PRs for repository {}/{}: {}",
                repo.org,
                repo.repo,
                error
            );
        }

        // Navigation actions (translated from NavigationAction)
        PullRequestAction::NavigateNext => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if !repo_data.prs.is_empty() {
                    repo_data.selected_pr = (repo_data.selected_pr + 1) % repo_data.prs.len();
                }
            }
        }

        PullRequestAction::NavigatePrevious => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if !repo_data.prs.is_empty() {
                    repo_data.selected_pr = if repo_data.selected_pr == 0 {
                        repo_data.prs.len() - 1
                    } else {
                        repo_data.selected_pr - 1
                    };
                }
            }
        }

        PullRequestAction::NavigateToTop => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                repo_data.selected_pr = 0;
            }
        }

        PullRequestAction::NavigateToBottom => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if !repo_data.prs.is_empty() {
                    repo_data.selected_pr = repo_data.prs.len() - 1;
                }
            }
        }

        // Repository switching
        PullRequestAction::RepositoryNext => {
            let num_repos = state.repositories.len();
            if num_repos > 0 {
                state.selected_repository = (state.selected_repository + 1) % num_repos;
                log::debug!("Switched to repository {}", state.selected_repository);
            }
        }

        PullRequestAction::RepositoryPrevious => {
            let num_repos = state.repositories.len();
            if num_repos > 0 {
                state.selected_repository = if state.selected_repository == 0 {
                    num_repos - 1
                } else {
                    state.selected_repository - 1
                };
                log::debug!("Switched to repository {}", state.selected_repository);
            }
        }

        // Selection actions
        PullRequestAction::ToggleSelection => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                    let pr_number = pr.number;
                    if repo_data.selected_pr_numbers.contains(&pr_number) {
                        repo_data.selected_pr_numbers.remove(&pr_number);
                        log::debug!("Deselected PR #{}", pr_number);
                    } else {
                        repo_data.selected_pr_numbers.insert(pr_number);
                        log::debug!("Selected PR #{}", pr_number);
                    }
                    repo_data.selected_pr = (repo_data.selected_pr + 1) % repo_data.prs.len();
                }
            }
        }

        PullRequestAction::SelectAll => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                repo_data.selected_pr_numbers = repo_data.prs.iter().map(|pr| pr.number).collect();
                log::debug!("Selected all {} PRs", repo_data.selected_pr_numbers.len());
            }
        }

        PullRequestAction::DeselectAll => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                let count = repo_data.selected_pr_numbers.len();
                repo_data.selected_pr_numbers.clear();
                log::debug!("Deselected {} PRs", count);
            }
        }

        // Filter actions
        PullRequestAction::SetFilter(filter) => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                repo_data.current_filter = filter.clone();
                repo_data.selected_pr = 0; // Reset selection when filter changes
            }
        }

        PullRequestAction::ClearFilter => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                repo_data.current_filter = crate::state::PrFilter::All;
                repo_data.selected_pr = 0;
            }
        }

        // Operations that are handled by middleware (these just get dispatched)
        // The actual state changes happen via success/error callbacks
        PullRequestAction::OpenInBrowser
        | PullRequestAction::OpenInIDE
        | PullRequestAction::OpenBuildLogs
        | PullRequestAction::Refresh
        | PullRequestAction::CycleFilter
        | PullRequestAction::MergeRequest
        | PullRequestAction::RebaseRequest
        | PullRequestAction::ApproveRequest
        | PullRequestAction::CommentRequest
        | PullRequestAction::RequestChangesRequest
        | PullRequestAction::CloseRequest
        | PullRequestAction::RerunFailedJobs => {
            // These are request actions - handled by middleware
        }

        // Actions with message payloads - handled by middleware
        PullRequestAction::ApproveWithMessage { .. }
        | PullRequestAction::CommentOnPr { .. }
        | PullRequestAction::RequestChanges { .. }
        | PullRequestAction::ClosePrWithMessage { .. } => {
            // These are confirmation actions - handled by middleware
        }

        // Operation start actions (could set loading state if needed)
        PullRequestAction::MergeStart { .. }
        | PullRequestAction::RebaseStart { .. }
        | PullRequestAction::ApproveStart { .. }
        | PullRequestAction::CommentStart { .. }
        | PullRequestAction::RequestChangesStart { .. }
        | PullRequestAction::CloseStart { .. }
        | PullRequestAction::RerunStart { .. } => {
            // These could set operation-in-progress state if needed
        }

        // CI/Build status actions
        PullRequestAction::CheckBuildStatus { .. } => {
            // Handled by middleware - triggers async CI status fetch
        }

        PullRequestAction::BuildStatusUpdated {
            repo,
            pr_number,
            status,
        } => {
            // Find repo index
            let Some(repo_idx) = find_repo_idx(&state, repo) else {
                log::warn!(
                    "Reducer: Repository {}/{} not found when updating PR #{}",
                    repo.org,
                    repo.repo,
                    pr_number
                );
                return state;
            };
            // Update the PR's mergeable status with the fetched CI status
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if let Some(pr) = repo_data
                    .prs
                    .iter_mut()
                    .find(|p| p.number == *pr_number as usize)
                {
                    log::info!(
                        "Reducer: Updating PR #{} status from {:?} to {:?}",
                        pr_number,
                        pr.mergeable,
                        status
                    );
                    pr.mergeable = *status;
                } else {
                    log::warn!(
                        "Reducer: PR #{} not found in repo_data for {}/{}",
                        pr_number,
                        repo.org,
                        repo.repo
                    );
                }
            } else {
                log::warn!(
                    "Reducer: repo_data not found for {}/{} when updating PR #{}",
                    repo.org,
                    repo.repo,
                    pr_number
                );
            }
        }

        PullRequestAction::StatsUpdated {
            repo,
            pr_number,
            additions,
            deletions,
        } => {
            // Find repo index
            let Some(repo_idx) = find_repo_idx(&state, repo) else {
                log::warn!(
                    "Reducer: Repository {}/{} not found when updating PR #{} stats",
                    repo.org,
                    repo.repo,
                    pr_number
                );
                return state;
            };
            // Update the PR's additions/deletions
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if let Some(pr) = repo_data
                    .prs
                    .iter_mut()
                    .find(|p| p.number == *pr_number as usize)
                {
                    log::debug!(
                        "Reducer: Updating PR #{} stats: +{} -{}",
                        pr_number,
                        additions,
                        deletions
                    );
                    pr.additions = *additions;
                    pr.deletions = *deletions;
                } else {
                    log::warn!(
                        "Reducer: PR #{} not found in repo_data for {}/{}",
                        pr_number,
                        repo.org,
                        repo.repo
                    );
                }
            } else {
                log::warn!(
                    "Reducer: repo_data not found for {}/{} when updating PR #{} stats",
                    repo.org,
                    repo.repo,
                    pr_number
                );
            }
        }
    }

    state
}
