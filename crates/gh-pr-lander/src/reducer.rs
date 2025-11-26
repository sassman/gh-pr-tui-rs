use crate::actions::Action;
use crate::reducers::debug_console_reducer;
use crate::state::{ActiveView, AppState};

/// Reducer - pure function that produces new state from current state + action
/// This is the root reducer that orchestrates all sub-reducers
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    // Handle global actions first
    match action {
        Action::GlobalQuit if state.active_view == ActiveView::Main => {
            state.running = false;
            return state;
        }
        Action::GlobalActivateView(new_view) => state.active_view = *new_view,
        _ => {}
    }

    // Run sub-reducers for component-specific actions
    state.debug_console = debug_console_reducer::reduce(state.debug_console, action);

    state
}
