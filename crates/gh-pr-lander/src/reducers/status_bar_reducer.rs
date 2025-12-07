//! Status Bar Reducer
//!
//! Handles status bar state updates.

use crate::actions::StatusBarAction;
use crate::state::{StatusBarState, StatusMessage};

/// Reduce status bar state
pub fn reduce_status_bar(mut state: StatusBarState, action: &StatusBarAction) -> StatusBarState {
    match action {
        StatusBarAction::Push {
            kind,
            message,
            source,
        } => {
            state.push(StatusMessage::new(*kind, message.clone(), source.clone()));
        }
        StatusBarAction::Clear => {
            state.clear();
        }
    }
    state
}
