//! Middleware system for Redux architecture
//!
//! Middleware sits between action dispatch and reducer execution, allowing
//! side effects, async operations, logging, and other cross-cutting concerns
//! to be handled in a composable way.
//!
//! ## Design
//!
//! ```text
//! Action → Middleware Chain → Reducer → State
//! ```
//!
//! Each middleware can:
//! - Inspect actions and state
//! - Dispatch new actions
//! - Perform side effects (async operations, logging, etc.)
//! - Block actions from reaching the reducer
//!
//! ## Example
//!
//! ```rust
//! struct LoggingMiddleware;
//!
//! impl Middleware for LoggingMiddleware {
//!     fn handle(
//!         &mut self,
//!         action: &Action,
//!         _state: &AppState,
//!         _dispatcher: &Dispatcher,
//!     ) -> BoxFuture<'_, bool> {
//!         Box::pin(async move {
//!             log::debug!("Action: {:?}", action);
//!             true // Continue to next middleware
//!         })
//!     }
//! }
//! ```

use crate::{actions::Action, state::AppState};
use std::future::Future;
use std::pin::Pin;

// Module declarations
mod dispatcher;
mod logging;
mod task;

// Re-exports
pub use dispatcher::Dispatcher;
pub use logging::LoggingMiddleware;
pub use task::TaskMiddleware;

/// BoxFuture type alias for async middleware handlers
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Middleware trait - handles actions before they reach the reducer
///
/// Middleware is called in order for each action. Each middleware can:
/// - Inspect the action and current state
/// - Dispatch new actions via the Dispatcher
/// - Perform async operations
/// - Block the action from continuing (return false)
///
/// # Example
///
/// ```rust
/// struct MyMiddleware;
///
/// impl Middleware for MyMiddleware {
///     fn handle<'a>(
///         &'a mut self,
///         action: &'a Action,
///         state: &'a AppState,
///         dispatcher: &'a Dispatcher,
///     ) -> BoxFuture<'a, bool> {
///         Box::pin(async move {
///             match action {
///                 Action::SomeAction => {
///                     // Perform side effect
///                     do_something().await;
///                     // Dispatch follow-up action
///                     dispatcher.dispatch(Action::SomeOtherAction);
///                     // Let action continue to reducer
///                     true
///                 }
///                 _ => true, // Pass through other actions
///             }
///         })
///     }
/// }
/// ```
pub trait Middleware: Send + Sync {
    /// Handle an action before it reaches the reducer
    ///
    /// # Parameters
    /// - `action`: The action being dispatched
    /// - `state`: Current application state (read-only)
    /// - `dispatcher`: Can dispatch new actions
    ///
    /// # Returns
    /// - `true`: Continue to next middleware/reducer
    /// - `false`: Block this action from continuing
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    struct TestMiddleware {
        called: bool,
    }

    impl Middleware for TestMiddleware {
        fn handle<'a>(
            &'a mut self,
            _action: &'a Action,
            _state: &'a AppState,
            _dispatcher: &'a Dispatcher,
        ) -> BoxFuture<'a, bool> {
            Box::pin(async move {
                self.called = true;
                true
            })
        }
    }

    #[tokio::test]
    async fn test_middleware_trait() {
        let mut middleware = TestMiddleware { called: false };
        let (tx, _rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);
        let state = AppState::default();

        let should_continue = middleware
            .handle(&Action::None, &state, &dispatcher)
            .await;

        assert!(should_continue);
        assert!(middleware.called);
    }
}
