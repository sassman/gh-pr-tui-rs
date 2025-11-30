//! PR Reducer
//!
//! Handles state updates for Pull Request data.

use crate::actions::Action;
use crate::domain_models::LoadingState;
use crate::state::MainViewState;

/// Reduce PR-related state based on actions
pub fn reduce(mut state: MainViewState, action: &Action) -> MainViewState {
    match action {
        Action::PrLoadStart(repo_idx) => {
            // Set loading state for the repository
            let repo_data = state.repo_data.entry(*repo_idx).or_default();
            repo_data.loading_state = LoadingState::Loading;
            log::debug!("PR loading started for repository {}", repo_idx);
        }

        Action::PrLoaded(repo_idx, prs) => {
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

        Action::PrLoadError(repo_idx, error) => {
            // Set error state for the repository
            let repo_data = state.repo_data.entry(*repo_idx).or_default();
            repo_data.loading_state = LoadingState::Error(error.clone());
            log::error!("Failed to load PRs for repository {}: {}", repo_idx, error);
        }

        Action::PrNavigateNext => {
            // Navigate to next PR in the current repository
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if !repo_data.prs.is_empty() {
                    repo_data.selected_pr = (repo_data.selected_pr + 1) % repo_data.prs.len();
                    log::debug!("PR navigation: selected PR {}", repo_data.selected_pr);
                }
            }
        }

        Action::PrNavigatePrevious => {
            // Navigate to previous PR in the current repository
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if !repo_data.prs.is_empty() {
                    repo_data.selected_pr = if repo_data.selected_pr == 0 {
                        repo_data.prs.len() - 1
                    } else {
                        repo_data.selected_pr - 1
                    };
                    log::debug!("PR navigation: selected PR {}", repo_data.selected_pr);
                }
            }
        }

        // NavigateNext/NavigatePrevious can also be used for PR navigation
        // when the main view has focus (handled by keyboard middleware)
        Action::NavigateNext => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                if !repo_data.prs.is_empty() {
                    repo_data.selected_pr = (repo_data.selected_pr + 1) % repo_data.prs.len();
                }
            }
        }

        Action::NavigatePrevious => {
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

        Action::PrToggleSelection => {
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
                }
            }
        }

        Action::PrSelectAll => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                repo_data.selected_pr_numbers = repo_data
                    .prs
                    .iter()
                    .map(|pr| pr.number)
                    .collect();
                log::debug!("Selected all {} PRs", repo_data.selected_pr_numbers.len());
            }
        }

        Action::PrDeselectAll => {
            let repo_idx = state.selected_repository;
            if let Some(repo_data) = state.repo_data.get_mut(&repo_idx) {
                let count = repo_data.selected_pr_numbers.len();
                repo_data.selected_pr_numbers.clear();
                log::debug!("Deselected {} PRs", count);
            }
        }

        _ => {}
    }

    state
}
