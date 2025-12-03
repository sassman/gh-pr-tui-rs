use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::state::AppState;

pub mod app_config_middleware;
pub mod bootstrap_middleware;
pub mod command_palette_middleware;
pub mod confirmation_popup_middleware;
pub mod github_middleware;
pub mod keyboard_middleware;
pub mod logging_middleware;
pub mod navigation_middleware;
pub mod pull_request_middleware;
pub mod repository_middleware;
pub mod text_input_middleware;

/// Middleware trait - intercepts actions before they reach the reducer
pub trait Middleware {
    /// Handle an action
    /// Returns true if the action should continue to the next middleware/reducer
    /// Returns false if the action should be consumed (not passed further)
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool;
}
