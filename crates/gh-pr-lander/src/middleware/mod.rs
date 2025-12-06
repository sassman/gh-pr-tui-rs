use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::state::AppState;

pub mod app_config_middleware;
pub mod bootstrap_middleware;
pub mod command_palette_middleware;
pub mod confirmation_popup_middleware;
pub mod debug_console_middleware;
pub mod github_middleware;
pub mod keyboard_middleware;
pub mod navigation_middleware;
pub mod pull_request_middleware;
pub mod repository_middleware;
pub mod text_input_middleware;

/// Middleware trait - intercepts actions before they reach the reducer
///
/// Middleware runs on the background thread, so it can perform blocking operations
/// (API calls, file I/O) without affecting the UI render loop.
pub trait Middleware: Send {
    /// Handle an action
    ///
    /// - `action`: The action to process
    /// - `state`: Current application state (read-only snapshot)
    /// - `dispatcher`: Use to dispatch actions that should re-enter middleware chain
    ///
    /// Returns `true` to continue chain, `false` to consume action
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool;
}
