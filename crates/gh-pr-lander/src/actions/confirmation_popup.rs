//! Confirmation Popup actions
//!
//! Actions specific to the confirmation popup overlay.
//! The popup is reusable for various PR operations (approve, comment, request changes, close).

use crate::state::ConfirmationIntent;

/// Actions for the Confirmation Popup screen
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmationPopupAction {
    /// Show the confirmation popup with the given configuration
    Show {
        intent: ConfirmationIntent,
        default_message: String,
        repo_context: String,
    },

    // Text input (translated from TextInputAction)
    /// Character typed into the message field
    Char(char),
    /// Backspace pressed - delete last character
    Backspace,
    /// Clear the entire message field
    ClearInput,

    // Control actions
    /// User pressed confirm - triggers intent dispatch (handled by middleware)
    Confirm,
    /// Intent has been dispatched - close the popup (handled by reducer)
    Confirmed,
    /// Cancel and close the popup (Esc, x, q)
    Cancel,
}
