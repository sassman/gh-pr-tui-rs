//! PR Reducer
//!
//! Handles state updates for Pull Request data using tagged PullRequestAction.

use crate::actions::PullRequestAction;
use crate::domain_models::LoadingState;
use crate::state::MainViewState;

/// Reduce PR-related state based on actions (new tagged action version)
///
/// Accepts only PullRequestAction, making it type-safe and focused.
pub fn reduce_pull_request(mut state: MainViewState, action: &PullRequestAction) -> MainViewState {
    match action {
        PullRequestAction::LoadStart(repo_idx) => {
            // Set loading state for the repository
            let repo_data = state.repo_data.entry(*repo_idx).or_default();
            repo_data.loading_state = LoadingState::Loading;
            log::debug!("PR loading started for repository {}", repo_idx);
        }

        PullRequestAction::Loaded(repo_idx, prs) => {
            // Update repository data with loaded PRs
            let repo_data = state.repo_data.entry(*repo_idx).or_default();
            repo_data.prs = prs.clone();
            repo_data.loading_state = LoadingState::Loaded;
            repo_data.last_updated = Some(chrono::Local::now());
            repo_data.selected_pr = 0;
            // Clear selection when PRs are reloaded
            repo_data.selected_pr_numbers.clear();
            log::info!("Loaded {} PRs for repository {}", prs.len(), repo_idx);
        }

        PullRequestAction::LoadError(repo_idx, error) => {
            // Set error state for the repository
            let repo_data = state.repo_data.entry(*repo_idx).or_default();
            repo_data.loading_state = LoadingState::Error(error.clone());
            log::error!("Failed to load PRs for repository {}: {}", repo_idx, error);
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

        // Merge operation state changes
        PullRequestAction::MergeStart(_repo_idx, _pr_idx) => {
            // Could set a "merging" flag if needed
        }
        PullRequestAction::MergeSuccess(_repo_idx, _pr_idx) => {
            // Middleware will trigger a refresh
        }
        PullRequestAction::MergeError(_repo_idx, _pr_idx, error) => {
            log::error!("Merge failed: {}", error);
        }

        // Rebase operation state changes
        PullRequestAction::RebaseStart(_repo_idx, _pr_idx) => {}
        PullRequestAction::RebaseSuccess(_repo_idx, _pr_idx) => {}
        PullRequestAction::RebaseError(_repo_idx, _pr_idx, error) => {
            log::error!("Rebase failed: {}", error);
        }

        // Approve operation state changes
        PullRequestAction::ApproveStart(_repo_idx, _pr_idx) => {}
        PullRequestAction::ApproveSuccess(_repo_idx, _pr_idx) => {}
        PullRequestAction::ApproveError(_repo_idx, _pr_idx, error) => {
            log::error!("Approve failed: {}", error);
        }

        // Comment operation state changes
        PullRequestAction::CommentStart(_repo_idx, _pr_idx) => {}
        PullRequestAction::CommentSuccess(_repo_idx, _pr_idx) => {}
        PullRequestAction::CommentError(_repo_idx, _pr_idx, error) => {
            log::error!("Comment failed: {}", error);
        }

        // Request changes operation state changes
        PullRequestAction::RequestChangesStart(_repo_idx, _pr_idx) => {}
        PullRequestAction::RequestChangesSuccess(_repo_idx, _pr_idx) => {}
        PullRequestAction::RequestChangesError(_repo_idx, _pr_idx, error) => {
            log::error!("Request changes failed: {}", error);
        }

        // Close operation state changes
        PullRequestAction::CloseStart(_repo_idx, _pr_idx) => {}
        PullRequestAction::CloseSuccess(_repo_idx, _pr_idx) => {}
        PullRequestAction::CloseError(_repo_idx, _pr_idx, error) => {
            log::error!("Close failed: {}", error);
        }

        // Rerun jobs state changes
        PullRequestAction::RerunStart(_repo_idx, _check_suite_id, _check_run_id) => {}
        PullRequestAction::RerunSuccess(_repo_idx, _check_suite_id, _check_run_id) => {}
        PullRequestAction::RerunError(_repo_idx, _check_suite_id, _check_run_id, error) => {
            log::error!("Rerun failed: {}", error);
        }

        // Open repository in browser - handled by middleware
        PullRequestAction::OpenRepositoryInBrowser => {}
    }

    state
}
