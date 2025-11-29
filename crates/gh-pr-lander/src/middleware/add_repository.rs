//! Add Repository Middleware
//!
//! Handles view management for the add repository form:
//! - Opening the form view on RepositoryAdd
//! - Closing the form view on AddRepoClose/AddRepoConfirm
//! - Translating generic TextInput actions to AddRepo-specific actions

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::{AddRepositoryView, ViewId};

/// Middleware that handles add repository form interactions
pub struct AddRepositoryMiddleware;

impl AddRepositoryMiddleware {
    pub fn new() -> Self {
        Self
    }

    /// Check if the add repository view is the active view
    fn is_active(state: &AppState) -> bool {
        state.active_view().view_id() == ViewId::AddRepository
    }
}

impl Default for AddRepositoryMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for AddRepositoryMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Handle opening the add repository view
            Action::RepositoryAdd => {
                log::debug!("Opening add repository form");
                // Push the view - the reducer will reset the form state
                dispatcher.dispatch(Action::PushView(Box::new(AddRepositoryView::new())));
                true // Let action pass through to reducer to reset form
            }

            // Handle closing the add repository view
            Action::AddRepoClose => {
                if Self::is_active(state) && state.view_stack.len() > 1 {
                    log::debug!("Closing add repository form");
                    dispatcher.dispatch(Action::GlobalClose);
                }
                true // Let action pass through to reducer to reset form
            }

            // Handle confirm - close view if form is valid
            Action::AddRepoConfirm => {
                if Self::is_active(state) && state.add_repo_form.is_valid() {
                    // Close the view after successful add
                    if state.view_stack.len() > 1 {
                        dispatcher.dispatch(Action::GlobalClose);
                    }
                }
                true // Let action pass through to reducer to add repository
            }

            // The rest only applies when the add repository view is active
            _ if !Self::is_active(state) => true,

            // Translate generic TextInput actions to AddRepo-specific actions
            Action::TextInputChar(c) => {
                dispatcher.dispatch(Action::AddRepoChar(*c));
                false // Consume the original action
            }

            Action::TextInputBackspace => {
                dispatcher.dispatch(Action::AddRepoBackspace);
                false
            }

            Action::TextInputClearLine => {
                dispatcher.dispatch(Action::AddRepoClearField);
                false
            }

            Action::TextInputEscape => {
                dispatcher.dispatch(Action::AddRepoClose);
                false
            }

            Action::TextInputConfirm => {
                dispatcher.dispatch(Action::AddRepoConfirm);
                false
            }

            // Tab navigation between fields
            Action::NavigateNext => {
                dispatcher.dispatch(Action::AddRepoNextField);
                false
            }

            Action::NavigatePrevious => {
                dispatcher.dispatch(Action::AddRepoPrevField);
                false
            }

            // All other actions pass through
            _ => true,
        }
    }
}
