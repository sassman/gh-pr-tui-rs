//! Key Bindings Panel Reducer
//!
//! Handles state updates for the key bindings help panel.

use crate::actions::KeyBindingsAction;
use crate::state::KeyBindingsPanelState;

/// Reducer for key bindings panel state
///
/// Accepts only KeyBindingsAction, making it type-safe and focused.
pub fn reduce_key_bindings(
    mut state: KeyBindingsPanelState,
    action: &KeyBindingsAction,
) -> KeyBindingsPanelState {
    match action {
        KeyBindingsAction::NavigateNext => {
            state.scroll_offset = state.scroll_offset.saturating_add(1);
        }
        KeyBindingsAction::NavigatePrevious => {
            // Note: max_scroll should be enforced by view model
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
        }
        KeyBindingsAction::NavigateToTop => {
            state.scroll_offset = 0;
        }
        KeyBindingsAction::NavigateToBottom => {
            // Could be set to max_scroll if we had it, but for now just stay at current
            // View model will clamp it anyway
        }
        KeyBindingsAction::Close => {
            // Reset scroll when closing
            state.scroll_offset = 0;
        }
    }
    state
}
