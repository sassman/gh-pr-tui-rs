//! Debug Console Reducer

use crate::actions::DebugConsoleAction;
use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::state::DebugConsoleState;

/// Reducer for debug console state.
///
/// Accepts only DebugConsoleAction, making it type-safe and focused.
pub fn reduce_debug_console(
    mut state: DebugConsoleState,
    action: &DebugConsoleAction,
) -> DebugConsoleState {
    let max_scroll = if state.visible_height > 0 {
        state.lines.len().saturating_sub(state.visible_height)
    } else {
        state.lines.len()
    };

    match action {
        DebugConsoleAction::NavigateNext => {
            // Scroll towards newer logs (decrease offset, towards 0)
            state.scroll_offset = state.scroll_offset.min(max_scroll);
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
        }
        DebugConsoleAction::NavigatePrevious => {
            // Scroll towards older logs (increase offset, capped at max_scroll)
            if state.scroll_offset < max_scroll {
                state.scroll_offset = state.scroll_offset.saturating_add(1);
            }
        }
        DebugConsoleAction::NavigateToTop => {
            // Go to oldest logs
            state.scroll_offset = max_scroll;
        }
        DebugConsoleAction::NavigateToBottom => {
            // Go to newest logs (offset = 0)
            state.scroll_offset = 0;
        }
        DebugConsoleAction::Clear => {
            state.lines.clear();
            state.scroll_offset = 0;
        }
        DebugConsoleAction::SetVisibleHeight(height) => {
            state.visible_height = *height;
        }
        DebugConsoleAction::LinesUpdated(new_lines) => {
            // Append delta to ring buffer (handles capacity internally)
            state.append_lines(new_lines.clone());
            // Keep scroll position valid
            let new_max = state.lines.len().saturating_sub(state.visible_height);
            state.scroll_offset = state.scroll_offset.min(new_max);
        }
    }
    state
}

impl PanelCapabilityProvider for DebugConsoleState {
    fn capabilities(&self) -> PanelCapabilities {
        // Debug console supports vim navigation and vertical scrolling with vim bindings
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
            | PanelCapabilities::SCROLL_VERTICAL
            | PanelCapabilities::VIM_SCROLL_BINDINGS
    }
}
