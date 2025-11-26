use crate::actions::Action;
use crate::reducers::{debug_console_reducer, splash_reducer};
use crate::state::AppState;
use crate::views::MainView;

/// Reducer - pure function that produces new state from current state + action
/// This is the root reducer that orchestrates all sub-reducers
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    // Handle global actions first
    match action {
        Action::GlobalQuit => {
            // Quit from any view
            state.running = false;
            return state;
        }
        Action::PushView(new_view) => {
            // Push a new view onto the stack (for modals/popups)
            // Check if this view is already the top-most view (prevent duplicates)
            let is_duplicate = state
                .view_stack
                .last()
                .map(|top| top.view_id() == new_view.view_id())
                .unwrap_or(false);

            if is_duplicate {
                log::debug!(
                    "Poping view from the stack, because this view is on top already: {:?}",
                    new_view.view_id()
                );
                state.view_stack.pop();
            } else {
                log::debug!("Pushing view onto stack: {:?}", new_view.view_id());
                state.view_stack.push(new_view.clone());
            }
        }
        Action::ReplaceView(new_view) => {
            // Replace entire view stack with new view (for navigation)
            log::debug!("Replacing view stack with: {:?}", new_view.view_id());
            state.view_stack.clear();
            state.view_stack.push(new_view.clone());
        }
        Action::GlobalClose => {
            // Close the top-most view
            // If there's more than one view in the stack, pop the top one
            // If there's only one view left, quit the application
            if state.view_stack.len() > 1 {
                let popped = state.view_stack.pop();
                log::debug!("Closed view: {:?}", popped.map(|v| v.view_id()));
            } else {
                log::debug!("Closing last view - quitting application");
                state.running = false;
            }
        }
        Action::BootstrapEnd => {
            // When bootstrap ends, switch to main view
            state.view_stack.clear();
            state.view_stack.push(Box::new(MainView::new()));
        }
        _ => {}
    }

    // Run sub-reducers for component-specific actions
    state.splash = splash_reducer::reduce(state.splash, action);
    state.debug_console = debug_console_reducer::reduce(state.debug_console, action);

    // Note: Capabilities are now computed on-demand via the View trait
    // instead of being stored in state

    state
}
