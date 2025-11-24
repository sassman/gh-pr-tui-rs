//! LoggingMiddleware - logs all actions for debugging

use super::{BoxFuture, Dispatcher, Middleware};
use crate::{actions::Action, state::AppState};

/// LoggingMiddleware - logs all actions that pass through the system
///
/// This is a simple example middleware that demonstrates the pattern.
/// It logs every action for debugging purposes.
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for LoggingMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        _state: &'a AppState,
        _dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            // Log the action (skip None to reduce noise)
            if !matches!(action, Action::None) {
                log::debug!("Action: {:?}", action);
            }
            // Always continue to next middleware
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_logging_middleware() {
        let mut middleware = LoggingMiddleware;
        let (tx, _rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);
        let state = AppState::default();

        let should_continue = middleware
            .handle(&Action::Quit, &state, &dispatcher)
            .await;

        assert!(should_continue);
    }
}
