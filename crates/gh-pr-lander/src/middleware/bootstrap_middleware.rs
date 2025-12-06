//! Bootstrap Middleware
//!
//! Manages application startup sequence:
//! - Dispatches LoadRecentRepositories to trigger repository loading
//! - Listens for LoadRecentRepositoriesDone to dispatch BootstrapEnd
//!
//! Note: Tick generation for splash animation is now handled by the background worker.

use crate::actions::{Action, BootstrapAction, GlobalAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::PullRequestView;

/// Bootstrap middleware - manages application startup
pub struct BootstrapMiddleware;

impl BootstrapMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BootstrapMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for BootstrapMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::Bootstrap(BootstrapAction::Start) => {
                log::info!("BootstrapMiddleware: Bootstrap starting");
                // NOTE: Repository loading is triggered by Event::ClientReady from github_middleware
                // NOTE: Tick generation for splash animation is handled by background worker
                true
            }

            Action::Bootstrap(BootstrapAction::LoadRecentRepositoriesDone) => {
                log::info!("BootstrapMiddleware: Repository loading done, ending bootstrap");
                dispatcher.dispatch(Action::Bootstrap(BootstrapAction::End));
                dispatcher.dispatch(Action::Global(GlobalAction::ReplaceView(Box::new(
                    PullRequestView::new(),
                ))));
                true
            }

            Action::Bootstrap(BootstrapAction::End) => {
                log::info!("BootstrapMiddleware: Bootstrap ended");
                true
            }

            _ => true,
        }
    }
}
