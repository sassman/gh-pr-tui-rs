use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::state::AppState;

pub mod bootstrap;
pub mod keyboard;
pub mod logging;
pub mod repository;

/// Middleware trait - intercepts actions before they reach the reducer
pub trait Middleware {
    /// Handle an action
    /// Returns true if the action should continue to the next middleware/reducer
    /// Returns false if the action should be consumed (not passed further)
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool;
}
