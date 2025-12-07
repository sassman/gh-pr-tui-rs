//! Pull Request Middleware
//!
//! Handles PR-specific side effects.
//!
//! Note: Actual GitHub API calls are handled by GitHubMiddleware.
//! Bulk loading coordination is handled by RepositoryMiddleware.

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// Middleware for Pull Request side effects
pub struct PullRequestMiddleware;

impl PullRequestMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PullRequestMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for PullRequestMiddleware {
    fn handle(&mut self, _action: &Action, _state: &AppState, _dispatcher: &Dispatcher) -> bool {
        // Currently all PR actions are handled by GitHubMiddleware
        // This middleware can be extended for PR-specific side effects
        true // Pass through all actions
    }
}
