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
use tokio::sync::mpsc;

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

/// Dispatcher allows middleware to dispatch new actions
///
/// Actions dispatched through the Dispatcher will be processed
/// in the next event loop iteration, preventing recursion.
#[derive(Clone)]
pub struct Dispatcher {
    tx: mpsc::UnboundedSender<Action>,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new(tx: mpsc::UnboundedSender<Action>) -> Self {
        Self { tx }
    }

    /// Dispatch an action
    ///
    /// The action will be queued and processed in the next iteration
    /// of the event loop.
    pub fn dispatch(&self, action: Action) {
        if let Err(e) = self.tx.send(action) {
            log::error!("Failed to dispatch action: {}", e);
        }
    }

    /// Dispatch an action from an async context
    ///
    /// This is useful when spawning tokio tasks that need to dispatch
    /// actions back to the store.
    pub fn dispatch_async(self, action: Action) {
        tokio::spawn(async move {
            self.dispatch(action);
        });
    }
}

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

/// TaskMiddleware - handles async operations like loading repos, merging PRs, etc.
///
/// This middleware replaces the old Effect/BackgroundTask system by handling
/// async operations directly in response to actions.
///
/// # Example Operations
/// - LoadRepo → fetch PR data from GitHub → dispatch RepoDataLoaded
/// - MergeSelectedPrs → call GitHub API → dispatch MergeComplete
/// - Rebase → call GitHub API → dispatch RebaseComplete
///
/// # Design
/// The middleware spawns tokio tasks for async operations and dispatches
/// result actions when complete. This eliminates the need for:
/// - Effect enum
/// - BackgroundTask enum
/// - TaskResult enum
/// - result_to_action() conversion
pub struct TaskMiddleware {
    // TODO: Add octocrab, cache, etc. when implementing specific handlers
    // octocrab: Option<octocrab::Octocrab>,
    // cache: Arc<Mutex<gh_api_cache::GitHubApiCache>>,
}

impl TaskMiddleware {
    pub fn new() -> Self {
        Self {
            // octocrab: None,
            // cache,
        }
    }
}

impl Default for TaskMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for TaskMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        _state: &'a AppState,
        _dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            match action {
                // Example: Handle Bootstrap action
                // In full implementation, this would:
                // 1. Load .env file
                // 2. Initialize Octocrab
                // 3. Load repositories
                // 4. Dispatch BootstrapComplete
                Action::Bootstrap => {
                    log::debug!("TaskMiddleware: Bootstrap action (not yet implemented)");
                    // For now, just pass through to let effect system handle it
                }

                // Example: Handle ReloadRepo action
                // In full implementation, this would:
                // 1. Spawn async task
                // 2. Fetch PR data from GitHub
                // 3. Dispatch RepoDataLoaded(idx, Ok(prs))
                Action::ReloadRepo(repo_index) => {
                    log::debug!(
                        "TaskMiddleware: ReloadRepo {} (not yet implemented)",
                        repo_index
                    );
                    // For now, just pass through to let effect system handle it
                }

                // All other actions pass through unchanged
                _ => {}
            }

            // Always continue to next middleware/reducer
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_dispatcher() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        dispatcher.dispatch(Action::None);

        let received = rx.try_recv();
        assert!(received.is_ok());
    }

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
