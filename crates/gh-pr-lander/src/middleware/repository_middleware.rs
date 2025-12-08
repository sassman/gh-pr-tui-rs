//! Repository Middleware
//!
//! Handles repository-related side effects:
//! - Loading recent repositories from config on LoadRecentRepositories
//! - Managing the add repository form view
//! - Translating generic TextInput actions to AddRepository-specific actions
//! - Opening repository URLs in the browser

use std::collections::HashSet;

use crate::actions::{
    Action, BootstrapAction, PullRequestAction, RepositoryAction, StatusBarAction,
};
use crate::dispatcher::Dispatcher;
use crate::domain_models::Repository;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::utils::browser::open_url;
use gh_pr_config::load_recent_repositories;
use tokio::runtime::Runtime;

/// Repository middleware - handles repository loading and add repository form
pub struct RepositoryMiddleware {
    /// Tokio runtime for async operations (opening URLs)
    runtime: Runtime,
    /// Track pending bulk load repository indices
    /// When all are loaded, we dispatch LoadRecentRepositoriesDone
    pending_bulk_load: HashSet<Repository>,
}

impl RepositoryMiddleware {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
            pending_bulk_load: HashSet::new(),
        }
    }

    /// Get the GitHub URL for the currently selected repository
    fn get_current_repo_url(state: &AppState) -> Option<String> {
        let repo_idx = state.main_view.selected_repository;
        state
            .main_view
            .repositories
            .get(repo_idx)
            .map(|repo| repo.web_url())
    }
    /// Mark a repository as done loading and check if bulk load is complete
    fn mark_bulk_load_done(&mut self, repo: Repository, dispatcher: &Dispatcher) {
        if self.pending_bulk_load.remove(&repo) {
            log::debug!(
                "PullRequestMiddleware: Repo {} done, {} remaining in bulk load",
                repo.full_display_name(),
                self.pending_bulk_load.len()
            );

            if self.pending_bulk_load.is_empty() {
                log::info!("PullRequestMiddleware: All bulk repositories loaded");
                dispatcher.dispatch(Action::Bootstrap(
                    BootstrapAction::LoadRecentRepositoriesDone,
                ));
            }
        }
    }
}

impl Default for RepositoryMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for RepositoryMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Handle loading recent repositories from config
            Action::Bootstrap(BootstrapAction::LoadRecentRepositories) => {
                log::info!("RepositoryMiddleware: Loading recent repositories from config");

                let recent_repos = load_recent_repositories();
                if !recent_repos.is_empty() {
                    let repositories: Vec<Repository> = recent_repos
                        .into_iter()
                        .map(|r| Repository::with_host(r.org, r.repo, r.branch, r.host))
                        .collect();
                    log::info!(
                        "RepositoryMiddleware: Found {} recent repositories",
                        repositories.len()
                    );
                    log::info!("Adding {} repositories from config", repositories.len());

                    for repo in repositories.into_iter() {
                        dispatcher.dispatch(Action::Repository(RepositoryAction::AddRepository(
                            repo.clone(),
                        )));

                        // when does it actually get removed?
                        self.pending_bulk_load.insert(repo.clone());

                        dispatcher.dispatch(Action::Repository(
                            RepositoryAction::LoadRepositoryData(repo.clone()),
                        ));
                    }
                } else {
                    log::info!("RepositoryMiddleware: No recent repositories found");
                    // Even if no repos, signal that loading is done
                    dispatcher.dispatch(Action::Bootstrap(
                        BootstrapAction::LoadRecentRepositoriesDone,
                    ));
                }

                true // Let action pass through
            }

            // When a single repository is added via form confirm
            Action::Repository(RepositoryAction::FormConfirm) => {
                if state.add_repo_form.is_valid() {
                    let repo = state.add_repo_form.to_repository();
                    // First add the repository to the list
                    dispatcher.dispatch(Action::Repository(RepositoryAction::AddRepository(
                        repo.clone(),
                    )));
                    // Then load its data (PRs, etc.)
                    dispatcher.dispatch(Action::Repository(RepositoryAction::LoadRepositoryData(
                        repo,
                    )));
                    // Note: View closing is handled by the reducer, not here
                }

                true // Let action pass through to reducer
            }

            // Handle opening repository in browser
            Action::Repository(RepositoryAction::OpenRepositoryInBrowser) => {
                if let Some(url) = Self::get_current_repo_url(state) {
                    let repo_idx = state.main_view.selected_repository;
                    let repo_name = state
                        .main_view
                        .repositories
                        .get(repo_idx)
                        .map(|r| format!("{}/{}", r.org, r.repo))
                        .unwrap_or_else(|| "repository".to_string());

                    log::info!("Opening repository {} in browser: {}", repo_name, url);

                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::info(
                        format!("Opening {} in browser", repo_name),
                        "Open Repository",
                    )));

                    self.runtime.spawn(open_url(url));
                } else {
                    log::warn!("No repository selected to open in browser");
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        "No repository selected",
                        "Open Repository",
                    )));
                }
                false // Consume action
            }

            // Handle PR loaded - check if bulk load is complete
            Action::PullRequest(PullRequestAction::Loaded { repo, .. }) => {
                self.mark_bulk_load_done(repo.clone(), dispatcher);
                true // Let action pass through to reducer
            }

            // Handle PR load error - still counts as "done" for bulk tracking
            Action::PullRequest(PullRequestAction::LoadError { repo, .. }) => {
                self.mark_bulk_load_done(repo.clone(), dispatcher);
                true // Let action pass through to reducer
            }

            // All other actions pass through
            _ => true,
        }
    }
}
