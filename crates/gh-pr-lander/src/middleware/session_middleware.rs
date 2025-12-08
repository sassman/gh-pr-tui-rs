//! Session Middleware
//!
//! Handles loading session on bootstrap and saving on quit.
//!
//! # Session Persistence
//!
//! - Loads session from disk during bootstrap
//! - Dispatches `Session::Loaded` action to store pending selection
//! - Dispatches `Session::RestoreSelection` after repositories load
//! - Saves session on quit
//! - Uses local session file if it exists, otherwise global

use crate::actions::{Action, BootstrapAction, GlobalAction, SessionAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_pr_config::{save_recent_repositories, RecentRepository, Session};
use std::sync::{Arc, Mutex};

/// Middleware for session state persistence
pub struct SessionMiddleware {
    session: Arc<Mutex<Session>>,
    loaded: bool,
}

impl SessionMiddleware {
    pub fn new() -> Self {
        Self {
            session: Arc::new(Mutex::new(Session::default())),
            loaded: false,
        }
    }

    fn save_session(&self, state: &AppState) {
        let mut session = self.session.lock().unwrap();
        let selected_idx = state.main_view.selected_repository;

        // Save current selection
        if let Some(repo) = state.main_view.repositories.get(selected_idx) {
            session.set_selected_repo(&repo.org, &repo.repo, &repo.branch, repo.host.as_deref());

            // Save selected PR number (not index) for this repository
            if let Some(repo_data) = state.main_view.repo_data.get(&selected_idx) {
                if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                    session.set_selected_pr_no(pr.number);
                }
            }
        }

        if let Err(e) = session.save() {
            log::error!("Failed to save session: {}", e);
        }
    }

    fn save_repositories(&self, state: &AppState) {
        let repos: Vec<RecentRepository> = state
            .main_view
            .repositories
            .iter()
            .map(|r| RecentRepository::with_host(&r.org, &r.repo, &r.branch, r.host.clone()))
            .collect();

        if let Err(e) = save_recent_repositories(&repos) {
            log::error!("Failed to save recent repositories: {}", e);
        }
    }
}

impl Default for SessionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for SessionMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::Bootstrap(BootstrapAction::Start) => {
                if !self.loaded {
                    // Run migrations before loading session
                    gh_pr_config_migrate::run_migrations();

                    log::info!("SessionMiddleware: Loading session");
                    let session = Session::load();

                    // Dispatch session loaded action with selected repo info
                    let selected_repo = session.selected_repo().map(|(org, name, branch, host)| {
                        (org.to_string(), name.to_string(), branch.to_string(), host.map(|h| h.to_string()))
                    });
                    let selected_pr_no = session.selected_pr_no();

                    dispatcher.dispatch(Action::Session(SessionAction::Loaded {
                        selected_repo,
                        selected_pr_no,
                    }));

                    *self.session.lock().unwrap() = session;
                    self.loaded = true;
                }
                true // Pass through
            }

            // Trigger session restore after repositories are loaded
            Action::Bootstrap(BootstrapAction::LoadRecentRepositoriesDone) => {
                dispatcher.dispatch(Action::Session(SessionAction::RestoreSelection));
                true // Pass through
            }

            // Save session and repositories on on close, when at root view
            Action::Global(GlobalAction::Close) if state.view_stack.len() == 1 => {
                log::info!("SessionMiddleware: Saving state before quit");
                self.save_session(state);
                self.save_repositories(state);
                true
            }

            _ => true,
        }
    }
}
