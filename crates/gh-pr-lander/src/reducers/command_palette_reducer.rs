//! Command palette reducer
//!
//! Handles CommandPalette-specific actions. This reducer only processes
//! actions prefixed with CommandPalette*, so it doesn't need to check
//! if the command palette is active - that's the middleware's job.

use crate::actions::Action;
use crate::commands::{filter_commands, get_palette_commands_with_hints};
use crate::keybindings::Keymap;
use crate::state::CommandPaletteState;

/// Reducer for command palette state
///
/// This is a pure state transformation - no side effects or dispatching.
/// Only handles CommandPalette* actions.
pub fn reduce(
    mut state: CommandPaletteState,
    action: &Action,
    keymap: &Keymap,
) -> CommandPaletteState {
    match action {
        Action::CommandPaletteChar(c) => {
            state.query.push(*c);
            state.selected_index = 0;
        }

        Action::CommandPaletteBackspace => {
            state.query.pop();
            state.selected_index = 0;
        }

        Action::CommandPaletteClose => {
            state.query.clear();
            state.selected_index = 0;
        }

        Action::CommandPaletteExecute => {
            // Just reset state - the middleware handles dispatching the command
            state.query.clear();
            state.selected_index = 0;
        }

        Action::CommandPaletteNavigateNext => {
            let all_commands = get_palette_commands_with_hints(keymap);
            let filtered = filter_commands(&all_commands, &state.query);
            if !filtered.is_empty() {
                state.selected_index = (state.selected_index + 1).min(filtered.len() - 1);
            }
        }

        Action::CommandPaletteNavigatePrev => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }

        _ => {}
    }

    state
}
