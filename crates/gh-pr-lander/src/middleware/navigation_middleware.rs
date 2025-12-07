//! Navigation Translation Middleware
//!
//! Translates generic Navigation actions into view-specific actions
//! using the active view's translate_navigation method.
//!
//! This ensures translated actions go through the full middleware chain.

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// Middleware that translates Navigation actions via the active view
pub struct NavigationMiddleware;

impl NavigationMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NavigationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for NavigationMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        if let Action::Navigate(nav) = action {
            if let Some(view) = state.view_stack.last() {
                if let Some(translated) = view.translate_navigation(*nav) {
                    log::debug!(
                        "NavigationMiddleware: Translating {:?} to {:?}",
                        nav,
                        translated
                    );
                    // Dispatch the translated action through the full middleware chain
                    dispatcher.dispatch(translated);
                    // Consume the original Navigate action
                    return false;
                }
            }
            log::debug!("Navigation action not handled by active view: {:?}", nav);
        }

        // Pass through all other actions
        true
    }
}
