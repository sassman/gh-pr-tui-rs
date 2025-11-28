use crate::actions::Action;
use crate::reducers::{command_palette_reducer, debug_console_reducer, splash_reducer};
use crate::state::AppState;
use crate::views::MainView;

/// Reducer - pure function that produces new state from current state + action
///
/// This is the root reducer that orchestrates all sub-reducers.
/// It handles truly global actions and delegates domain-specific actions
/// to the appropriate sub-reducers.
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    // Handle global actions first (these are view-agnostic)
    match action {
        Action::GlobalQuit => {
            state.running = false;
            return state;
        }

        Action::PushView(new_view) => {
            // Check if this view is already the top-most view (toggle behavior)
            let is_duplicate = state
                .view_stack
                .last()
                .map(|top| top.view_id() == new_view.view_id())
                .unwrap_or(false);

            if is_duplicate {
                log::debug!(
                    "Popping view from the stack, because this view is on top already: {:?}",
                    new_view.view_id()
                );
                state.view_stack.pop();
            } else {
                log::debug!("Pushing view onto stack: {:?}", new_view.view_id());
                state.view_stack.push(new_view.clone());
            }
        }

        Action::ReplaceView(new_view) => {
            log::debug!("Replacing view stack with: {:?}", new_view.view_id());
            state.view_stack.clear();
            state.view_stack.push(new_view.clone());
        }

        Action::GlobalClose | Action::CommandPaletteClose | Action::CommandPaletteExecute => {
            // Close the top-most view (CommandPaletteClose/Execute specifically close the palette)
            if state.view_stack.len() > 1 {
                let popped = state.view_stack.pop();
                log::debug!("Closed view: {:?}", popped.map(|v| v.view_id()));
            } else {
                log::debug!("Closing last view - quitting application");
                state.running = false;
            }
        }

        Action::BootstrapEnd => {
            state.view_stack.clear();
            state.view_stack.push(Box::new(MainView::new()));
        }

        Action::RepositoryNext => {
            let num_repos = 2; // TODO: Make this dynamic based on actual repositories
            state.main_view.selected_repository =
                (state.main_view.selected_repository + 1) % num_repos;
            log::debug!(
                "Switched to repository {}",
                state.main_view.selected_repository
            );
        }

        Action::RepositoryPrevious => {
            let num_repos = 2; // TODO: Make this dynamic based on actual repositories
            state.main_view.selected_repository = if state.main_view.selected_repository == 0 {
                num_repos - 1
            } else {
                state.main_view.selected_repository - 1
            };
            log::debug!(
                "Switched to repository {}",
                state.main_view.selected_repository
            );
        }

        _ => {}
    }

    // Run sub-reducers - each is responsible for checking if it should handle the action
    // based on the active view or other criteria

    // Splash reducer (simple state update)
    state.splash = splash_reducer::reduce(state.splash, action);

    // Debug console reducer (simple state update)
    state.debug_console = debug_console_reducer::reduce(state.debug_console, action);

    // Command palette reducer (handles CommandPalette* actions only)
    state.command_palette =
        command_palette_reducer::reduce(state.command_palette, action, &state.keymap);

    state
}
