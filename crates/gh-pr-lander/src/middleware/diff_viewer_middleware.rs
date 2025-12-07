//! Diff Viewer Middleware
//!
//! Handles side effects for the diff viewer, including the close logic
//! when Escape is pressed and there's nothing to cancel/escape from.

use crate::actions::{Action, DiffViewerAction, GlobalAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// Middleware for diff viewer side effects
pub struct DiffViewerMiddleware;

impl DiffViewerMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiffViewerMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for DiffViewerMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Only intercept DiffViewer EscapeOrFocusTree action
        if let Action::DiffViewer(DiffViewerAction::EscapeOrFocusTree) = action {
            // Check current state to decide if we should close the view
            // This runs BEFORE the reducer, so we check the pre-action state
            if let Some(ref inner) = state.diff_viewer.inner {
                if inner.is_editing_comment() {
                    // Let reducer handle: cancel comment
                    return true;
                }
                if inner.show_review_popup {
                    // Let reducer handle: hide review popup
                    return true;
                }
                if !inner.nav.file_tree_focused {
                    // Let reducer handle: focus file tree
                    return true;
                }
                // Already in file tree with nothing to cancel - close the view
                log::debug!("DiffViewerMiddleware: EscapeOrFocusTree - closing view");
                dispatcher.dispatch(Action::Global(GlobalAction::Close));
                return false; // Consume the action
            } else {
                // No inner state - close the view
                log::debug!(
                    "DiffViewerMiddleware: EscapeOrFocusTree with no inner - closing view"
                );
                dispatcher.dispatch(Action::Global(GlobalAction::Close));
                return false;
            }
        }

        // All other actions pass through
        true
    }
}
