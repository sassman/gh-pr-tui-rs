use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::reducers::app_reducer::reduce;
use crate::state::AppState;

/// Store - holds application state and manages the Redux loop
pub struct Store {
    state: AppState,
    middleware: Vec<Box<dyn Middleware>>,
    dispatcher: Dispatcher,
}

impl Store {
    pub fn new(initial_state: AppState) -> Self {
        Self {
            state: initial_state,
            middleware: Vec::new(),
            dispatcher: Dispatcher::new(),
        }
    }

    /// Add middleware to the store
    pub fn add_middleware(&mut self, middleware: Box<dyn Middleware>) {
        self.middleware.push(middleware);
    }

    /// Get the current state
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Get the dispatcher
    pub fn dispatcher(&self) -> &Dispatcher {
        &self.dispatcher
    }

    /// Process an action through middleware chain and reducer
    pub fn dispatch(&mut self, action: Action) {
        let mut should_reduce = true;

        // Pass through middleware chain
        for middleware in &mut self.middleware {
            if !middleware.handle(&action, &self.state, &self.dispatcher) {
                should_reduce = false;
                break;
            }
        }

        // If no middleware consumed the action, send to reducer
        if should_reduce {
            self.state = reduce(self.state.clone(), &action);
        }

        // Process any actions dispatched by middleware
        let pending_actions = self.dispatcher.drain();
        for action in pending_actions {
            self.dispatch(action);
        }
    }
}
