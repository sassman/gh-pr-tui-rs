use crate::actions::Action;
use crate::commands::{filter_commands, get_all_commands};
use crate::reducers::{command_palette_reducer, debug_console_reducer, splash_reducer};
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
        Action::RepositoryNext => {
            // Move to next repository (with wrapping)
            let num_repos = 2; // TODO: Make this dynamic based on actual repositories
            state.main_view.selected_repository =
                (state.main_view.selected_repository + 1) % num_repos;
            log::debug!(
                "Switched to repository {}",
                state.main_view.selected_repository
            );
        }
        Action::RepositoryPrevious => {
            // Move to previous repository (with wrapping)
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
        Action::CommandPaletteExecute => {
            // Execute the currently selected command
            let all_commands = get_all_commands();
            let filtered = filter_commands(&all_commands, &state.command_palette.query);

            if let Some(cmd) = filtered.get(state.command_palette.selected_index) {
                log::debug!("Executing command: {}", cmd.title);
                // Close the command palette first
                if state.view_stack.last().map(|v| v.view_id())
                    == Some(crate::views::ViewId::CommandPalette)
                {
                    state.view_stack.pop();
                }
                // Reset command palette state
                state.command_palette.query.clear();
                state.command_palette.selected_index = 0;
                // Dispatch the command's action by recursively calling reduce
                return reduce(state, &cmd.action);
            }
        }
        _ => {}
    }

    // Run sub-reducers for component-specific actions
    state.splash = splash_reducer::reduce(state.splash, action);
    state.debug_console = debug_console_reducer::reduce(state.debug_console, action);
    state.command_palette = command_palette_reducer::reduce(state.command_palette, action);

    // Note: Capabilities are now computed on-demand via the View trait
    // instead of being stored in state

    state
}
