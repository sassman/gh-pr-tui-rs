//! Text Input Translation Middleware
//!
//! Translates generic TextInput actions into view-specific actions
//! using the active view's translate_text_input method.
//!
//! This ensures translated actions go through the full middleware chain.

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// Middleware that translates TextInput actions via the active view
pub struct TextInputMiddleware;

impl TextInputMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TextInputMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for TextInputMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        if let Action::TextInput(input) = action {
            if let Some(view) = state.view_stack.last() {
                if let Some(translated) = view.translate_text_input(input.clone()) {
                    log::debug!(
                        "TextInputMiddleware: Translating {:?} to {:?}",
                        input,
                        translated
                    );
                    // Dispatch the translated action through the full middleware chain
                    dispatcher.dispatch(translated);
                    // Consume the original TextInput action
                    return false;
                }
            }
            log::debug!("TextInput action not handled by active view: {:?}", input);
        }

        // Pass through all other actions
        true
    }
}
