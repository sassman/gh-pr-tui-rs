//! Confirmation Popup Middleware
//!
//! Handles the Confirm action by extracting the intent and message from state,
//! then dispatching the appropriate PR action.

use crate::actions::{Action, ConfirmationPopupAction, PullRequestAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::{AppState, ConfirmationIntent};

/// Middleware that handles confirmation popup action dispatching
pub struct ConfirmationPopupMiddleware;

impl ConfirmationPopupMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfirmationPopupMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for ConfirmationPopupMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Handle Confirm action - dispatch the appropriate PR action based on intent
        if let Action::ConfirmationPopup(ConfirmationPopupAction::Confirm) = action {
            if let Some(popup) = &state.confirmation_popup {
                let message = popup.input_value.clone();
                let pr_action = match &popup.intent {
                    ConfirmationIntent::Approve { pr_numbers } => {
                        Action::PullRequest(PullRequestAction::ApproveWithMessage {
                            pr_numbers: pr_numbers.clone(),
                            message,
                        })
                    }
                    ConfirmationIntent::Comment { pr_numbers } => {
                        Action::PullRequest(PullRequestAction::CommentOnPr {
                            pr_numbers: pr_numbers.clone(),
                            message,
                        })
                    }
                    ConfirmationIntent::RequestChanges { pr_numbers } => {
                        Action::PullRequest(PullRequestAction::RequestChanges {
                            pr_numbers: pr_numbers.clone(),
                            message,
                        })
                    }
                    ConfirmationIntent::Close { pr_numbers } => {
                        Action::PullRequest(PullRequestAction::ClosePrWithMessage {
                            pr_numbers: pr_numbers.clone(),
                            message,
                        })
                    }
                };

                log::debug!(
                    "Confirmation popup confirmed, dispatching PR action: {:?}",
                    popup.intent
                );
                // Dispatch the PR action
                dispatcher.dispatch(pr_action);
                // Dispatch Confirmed to close the popup
                dispatcher.dispatch(Action::ConfirmationPopup(
                    ConfirmationPopupAction::Confirmed,
                ));
            }
            // Consume the Confirm action - we've dispatched what we need
            return false;
        }

        // All other actions pass through
        true
    }
}
