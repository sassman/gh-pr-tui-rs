//! Pull Request Middleware
//!
//! Handles coordination for bulk PR loading:
//! - Triggers PR loading when repositories are added
//! - Tracks bulk loading and dispatches LoadRecentRepositoriesDone when complete
//!
//! Note: Actual GitHub API calls are handled by GitHubMiddleware.
//! This middleware only coordinates the loading process.

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use std::collections::HashSet;

/// Middleware for coordinating Pull Request loading
pub struct PullRequestMiddleware {
    /// Track pending bulk load repository indices
    /// When all are loaded, we dispatch LoadRecentRepositoriesDone
    pending_bulk_load: HashSet<usize>,
}

impl PullRequestMiddleware {
    pub fn new() -> Self {
        Self {
            pending_bulk_load: HashSet::new(),
        }
    }

    /// Mark a repository as done loading and check if bulk load is complete
    fn mark_bulk_load_done(&mut self, repo_idx: usize, dispatcher: &Dispatcher) {
        if self.pending_bulk_load.remove(&repo_idx) {
            log::debug!(
                "PullRequestMiddleware: Repo {} done, {} remaining in bulk load",
                repo_idx,
                self.pending_bulk_load.len()
            );

            if self.pending_bulk_load.is_empty() {
                log::info!("PullRequestMiddleware: All bulk repositories loaded");
                dispatcher.dispatch(Action::LoadRecentRepositoriesDone);
            }
        }
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
            // When repositories are added in bulk, start loading PRs for each
            Action::RepositoryAddBulk(repos) => {
                log::info!(
                    "RepositoryAddBulk received with {} repos",
                    repos.len(),
                );

                // Calculate starting index (after existing repos)
                let start_idx = state.main_view.repositories.len();
                log::info!(
                    "Current repos in state: {}, will dispatch PrLoadStart for indices {}..{}",
                    start_idx,
                    start_idx,
                    start_idx + repos.len()
                );

                // Track all repos we're about to load
                for i in 0..repos.len() {
                    self.pending_bulk_load.insert(start_idx + i);
                }

                // Dispatch PrLoadStart for each new repository
                for (i, _repo) in repos.iter().enumerate() {
                    let repo_idx = start_idx + i;
                    dispatcher.dispatch(Action::PrLoadStart(repo_idx));
                }

                true // Let action pass through to reducer
            }

            // When a single repository is added via confirm
            Action::AddRepoConfirm => {
                if state.add_repo_form.is_valid() {
                    // The new repo will be at the end of the list
                    let repo_idx = state.main_view.repositories.len();
                    dispatcher.dispatch(Action::PrLoadStart(repo_idx));
                }

                true // Let action pass through to reducer
            }

            // Handle PR loaded - check if bulk load is complete
            Action::PrLoaded(repo_idx, _) => {
                self.mark_bulk_load_done(*repo_idx, dispatcher);
                true // Let action pass through to reducer
            }

            // Handle PR load error - still counts as "done" for bulk tracking
            Action::PrLoadError(repo_idx, _) => {
                self.mark_bulk_load_done(*repo_idx, dispatcher);
                true // Let action pass through to reducer
            }

            _ => true, // Pass through all other actions
        }
    }
}
