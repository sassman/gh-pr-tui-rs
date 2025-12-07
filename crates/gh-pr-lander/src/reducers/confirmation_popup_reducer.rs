//! Confirmation Popup Reducer
//!
//! Handles all state changes for the confirmation popup, including
//! view stack management and text input.

use crate::actions::ConfirmationPopupAction;
use crate::state::{AppState, ConfirmationPopupState};
use crate::views::ConfirmationPopupView;

/// Reduce confirmation popup state based on actions.
///
/// Handles all ConfirmationPopup actions including view stack management.
pub fn reduce_confirmation_popup(
    mut state: AppState,
    action: &ConfirmationPopupAction,
) -> AppState {
    match action {
        ConfirmationPopupAction::Show {
            intent,
            default_message,
            repo_context,
        } => {
            // Create popup state and push view
            state.confirmation_popup = Some(ConfirmationPopupState::new(
                intent.clone(),
                default_message.clone(),
                repo_context.clone(),
            ));
            state
                .view_stack
                .push(Box::new(ConfirmationPopupView::new()));
            log::debug!("Showing confirmation popup: {:?}", intent);
        }

        ConfirmationPopupAction::Cancel => {
            // Clear state and pop view
            state.confirmation_popup = None;
            if state.view_stack.len() > 1 {
                state.view_stack.pop();
            }
            log::debug!("Cancelled confirmation popup");
        }

        ConfirmationPopupAction::Confirm => {
            // Handled by middleware - should not reach reducer
        }

        ConfirmationPopupAction::Confirmed => {
            // Clear state and pop view after middleware dispatched PR action
            state.confirmation_popup = None;
            if state.view_stack.len() > 1 {
                state.view_stack.pop();
            }
            log::debug!("Confirmation popup closed after confirm");
        }

        ConfirmationPopupAction::Char(c) => {
            if let Some(ref mut popup) = state.confirmation_popup {
                popup.input_value.push(*c);
            }
        }

        ConfirmationPopupAction::Backspace => {
            if let Some(ref mut popup) = state.confirmation_popup {
                popup.input_value.pop();
            }
        }

        ConfirmationPopupAction::ClearInput => {
            if let Some(ref mut popup) = state.confirmation_popup {
                popup.input_value.clear();
            }
        }
    }

    state
}
