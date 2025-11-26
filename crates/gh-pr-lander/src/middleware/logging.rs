use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// LoggingMiddleware - logs all actions passing through
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for LoggingMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, _dispatcher: &Dispatcher) -> bool {
        // Log all actions (custom logger will send to debug console)
        // Don't log DebugConsoleLogAdded to avoid infinite loop
        if !matches!(action, Action::DebugConsoleLogAdded(_)) {
            log::debug!("Action: {:?}", action);
        }

        true // Always pass action through
    }
}
