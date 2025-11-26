use crate::actions::Action;
use crate::state::{ActiveView, DebugConsoleState};

/// Reducer for debug console state
pub fn reduce(mut state: DebugConsoleState, action: &Action) -> DebugConsoleState {
    let is_active = state.visible;
    match action {
        Action::GlobalActivateView(ActiveView::DebugConsole) => {
            state.visible = true;
        }
        Action::GlobalActivateView(_) => state.visible = false,
        Action::DebugConsoleLogAdded(msg) => {
            state.logs.push(msg.clone());
        }
        Action::LocalKeyPressed(c) if *c == 'c' && is_active => {
            // Handle local 'c' key - clear logs
            state.logs.clear();
        }
        Action::DebugConsoleClear if is_active => {
            state.logs.clear();
        }
        _ => {
            // Unhandled actions - no state change
        }
    }

    state
}
