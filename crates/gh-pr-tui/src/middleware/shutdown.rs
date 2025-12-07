//! ShutdownMiddleware - manages graceful application shutdown
//!
//! This middleware handles:
//! - Intercepting Quit and FatalError actions
//! - Performing cleanup (save repos, save session state)
//! - Closing the action channel to signal main loop termination
//! - Ensuring shutdown happens exactly once

use super::{BoxFuture, Dispatcher, Middleware};
use crate::{actions::Action, state::AppState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Middleware for managing graceful application shutdown
pub struct ShutdownMiddleware {
    /// Shared flag to signal main loop to exit
    should_quit: Arc<AtomicBool>,
    /// Flag to ensure shutdown happens exactly once
    shutdown_initiated: Arc<AtomicBool>,
}

impl ShutdownMiddleware {
    pub fn new(should_quit: Arc<AtomicBool>) -> Self {
        Self {
            should_quit,
            shutdown_initiated: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Perform cleanup and initiate shutdown
    async fn perform_shutdown(&self, state: &AppState, reason: &str) {
        // Ensure shutdown happens exactly once
        if self
            .shutdown_initiated
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            log::debug!("Shutdown already initiated, skipping duplicate shutdown");
            return;
        }

        log::info!(
            "ShutdownMiddleware: Initiating graceful shutdown ({})",
            reason
        );

        // Save recent repositories
        if let Err(e) = self.store_recent_repos(&state.repos.recent_repos) {
            log::error!("Failed to save recent repositories during shutdown: {}", e);
        }

        // Save session state (selected repo)
        if let Some(selected_repo) = state.repos.recent_repos.get(state.repos.selected_repo) {
            if let Err(e) = self.store_persisted_state(selected_repo) {
                log::error!("Failed to save session state during shutdown: {}", e);
            }
        }

        log::info!("ShutdownMiddleware: Cleanup complete, setting should_quit flag");

        // Set the should_quit flag - main loop will check this and exit
        self.should_quit.store(true, Ordering::SeqCst);
    }

    /// Save recent repositories to file
    fn store_recent_repos(&self, repos: &[crate::state::Repo]) -> anyhow::Result<()> {
        let file = gh_pr_config::create_recent_repositories_file()?;
        serde_json::to_writer_pretty(file, repos)?;
        log::debug!("Saved {} repositories", repos.len());
        Ok(())
    }

    /// Save session state to file
    fn store_persisted_state(&self, selected_repo: &crate::state::Repo) -> anyhow::Result<()> {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct PersistedState {
            selected_repo: crate::state::Repo,
        }

        let state = PersistedState {
            selected_repo: selected_repo.clone(),
        };

        let file = gh_pr_config::create_session_file()?;
        serde_json::to_writer_pretty(file, &state)?;
        log::debug!("Saved session state");
        Ok(())
    }
}

impl Middleware for ShutdownMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        _dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            match action {
                Action::Quit => {
                    log::debug!("ShutdownMiddleware: Quit action received");
                    self.perform_shutdown(state, "user quit").await;
                    // Don't pass Quit to reducer - we're handling it here
                    // Return false to block the action
                    return false;
                }
                Action::FatalError(err) => {
                    log::error!("ShutdownMiddleware: Fatal error - {}", err);
                    self.perform_shutdown(state, &format!("fatal error: {}", err))
                        .await;
                    // Don't pass FatalError to reducer
                    return false;
                }
                _ => {}
            }

            // Continue to next middleware for all other actions
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_shutdown_on_quit() {
        let should_quit = Arc::new(AtomicBool::new(false));
        let (dispatcher_tx, _dispatcher_rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(dispatcher_tx);

        let mut middleware = ShutdownMiddleware::new(should_quit.clone());
        let state = AppState::default();

        // Handle Quit action
        let should_continue = middleware.handle(&Action::Quit, &state, &dispatcher).await;

        // Should block the action (return false)
        assert!(!should_continue);

        // Should set the quit flag
        assert!(should_quit.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_shutdown_on_fatal_error() {
        let should_quit = Arc::new(AtomicBool::new(false));
        let (dispatcher_tx, _dispatcher_rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(dispatcher_tx);

        let mut middleware = ShutdownMiddleware::new(should_quit.clone());
        let state = AppState::default();

        // Handle FatalError action
        let should_continue = middleware
            .handle(
                &Action::FatalError("test error".to_string()),
                &state,
                &dispatcher,
            )
            .await;

        // Should block the action
        assert!(!should_continue);

        // Should set the quit flag
        assert!(should_quit.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_shutdown_happens_once() {
        let should_quit = Arc::new(AtomicBool::new(false));
        let (dispatcher_tx, _dispatcher_rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(dispatcher_tx);

        let mut middleware = ShutdownMiddleware::new(should_quit.clone());
        let state = AppState::default();

        // First quit
        middleware.handle(&Action::Quit, &state, &dispatcher).await;
        assert!(should_quit.load(Ordering::SeqCst));

        // Second quit - should be ignored (shutdown already initiated)
        middleware.handle(&Action::Quit, &state, &dispatcher).await;

        // If we got here without panic, the duplicate shutdown was handled gracefully
    }

    #[tokio::test]
    async fn test_other_actions_pass_through() {
        let should_quit = Arc::new(AtomicBool::new(false));
        let (dispatcher_tx, _dispatcher_rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(dispatcher_tx);

        let mut middleware = ShutdownMiddleware::new(should_quit.clone());
        let state = AppState::default();

        // Handle a non-shutdown action
        let should_continue = middleware
            .handle(&Action::Bootstrap, &state, &dispatcher)
            .await;

        // Should continue (return true)
        assert!(should_continue);

        // Should not set quit flag
        assert!(!should_quit.load(Ordering::SeqCst));
    }
}
