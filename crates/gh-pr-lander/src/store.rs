use crate::actions::Action;
use crate::reducers::app_reducer::reduce;
use crate::state::AppState;

/// Store - holds application state
///
/// In the new architecture, middleware runs on a background thread.
/// The store only handles reducer logic on the main thread.
pub struct Store {
    state: AppState,
}

impl Store {
    pub fn new(initial_state: AppState) -> Self {
        Self {
            state: initial_state,
        }
    }

    /// Get the current state
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Get mutable state reference
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    /// Process an action through reducer ONLY (no middleware)
    ///
    /// Middleware runs on background thread, so this is just reducer logic.
    pub fn dispatch(&mut self, action: Action) {
        self.state = reduce(self.state.clone(), &action);
    }
}
