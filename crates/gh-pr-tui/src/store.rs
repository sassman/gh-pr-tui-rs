use crate::{
    actions::Action, middleware::{Dispatcher, Middleware}, reducer::reduce,
    state::AppState,
};

/// Redux-style Store that holds application state and dispatches actions
///
/// The Store follows the Redux pattern:
/// - Centralized state management
/// - Actions are dispatched to modify state
/// - Pure reducers handle state transitions
/// - State is immutable (replaced on each action)
///
/// # Middleware Support (New)
///
/// The store now supports middleware for handling side effects:
/// ```rust
/// let mut store = Store::new(state);
/// store.add_middleware(LoggingMiddleware);
/// store.add_middleware(TaskMiddleware::new(octocrab, cache));
///
/// // Async dispatch through middleware
/// store.dispatch_async(action, &dispatcher).await;
/// ```
pub struct Store {
    state: AppState,
    middleware: Vec<Box<dyn Middleware>>,
}

impl Store {
    /// Create a new store with initial state
    pub fn new(initial_state: AppState) -> Self {
        Self {
            state: initial_state,
            middleware: Vec::new(),
        }
    }

    /// Add middleware to the store
    ///
    /// Middleware is called in the order it was added.
    /// Add middleware before starting the event loop.
    ///
    /// # Example
    /// ```rust
    /// store.add_middleware(LoggingMiddleware);
    /// store.add_middleware(TaskMiddleware::new(octocrab, cache));
    /// ```
    pub fn add_middleware<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middleware.push(Box::new(middleware));
    }

    /// Get immutable reference to current state
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Get mutable reference to current state
    /// Note: Direct mutation should be avoided - prefer dispatch() for state changes
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    /// Dispatch an action through middleware chain, then reducer
    ///
    /// This is the new way to dispatch actions. Actions flow through
    /// the middleware chain before reaching the reducer, allowing
    /// side effects to be handled cleanly.
    ///
    /// All side effects are now handled by middleware, so this method
    /// returns nothing.
    ///
    /// # Example
    /// ```rust
    /// store.dispatch_async(Action::MergeSelectedPrs, &dispatcher).await;
    /// ```
    pub async fn dispatch_async(
        &mut self,
        action: Action,
        dispatcher: &Dispatcher,
    ) {
        // Run action through middleware chain
        let mut should_continue = true;
        for middleware in &mut self.middleware {
            if !middleware.handle(&action, &self.state, dispatcher).await {
                should_continue = false;
                break;
            }
        }

        // If not blocked by middleware, apply to reducer
        if should_continue {
            let (new_state, _effects) = reduce(self.state.clone(), &action);
            self.state = new_state;
            // Effects are always empty now - all side effects in middleware
        }
    }

    /// Dispatch an action to update state (old method, kept for compatibility)
    ///
    /// This is the old synchronous dispatch method. It bypasses middleware
    /// and goes straight to the reducer.
    ///
    /// Prefer `dispatch_async()` for new code. This method should only be used
    /// in tests or simple scenarios where middleware is not needed.
    pub fn dispatch(&mut self, action: Action) {
        // Apply reducer to get new state (effects ignored)
        let (new_state, _effects) = reduce(self.state.clone(), &action);

        // Replace old state with new state
        self.state = new_state;
        // Effects are always empty now - all side effects in middleware
    }

    /// Dispatch an action by reference (useful when action should not be moved)
    pub fn dispatch_ref(&mut self, action: &Action) {
        let (new_state, _effects) = reduce(self.state.clone(), action);
        self.state = new_state;
        // Effects are always empty now - all side effects in middleware
    }

    /// Replace entire state (useful for initialization or testing)
    pub fn replace_state(&mut self, state: AppState) {
        self.state = state;
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new(AppState::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_dispatch_quit() {
        let mut store = Store::default();
        assert!(!store.state().ui.should_quit);

        store.dispatch(Action::Quit);
        assert!(store.state().ui.should_quit);
    }

    #[test]
    fn test_store_dispatch_toggle_shortcuts() {
        let mut store = Store::default();
        assert!(!store.state().ui.show_shortcuts);

        store.dispatch(Action::ToggleShortcuts);
        assert!(store.state().ui.show_shortcuts);

        store.dispatch(Action::ToggleShortcuts);
        assert!(!store.state().ui.show_shortcuts);
    }
}
