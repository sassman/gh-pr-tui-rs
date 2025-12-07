//! Command palette reducer
//!
//! Handles CommandPalette-specific actions. This reducer only processes
//! actions prefixed with CommandPalette*, so it doesn't need to check
//! if the command palette is active - that's the middleware's job.

use crate::actions::CommandPaletteAction;
use crate::commands::{filter_commands, get_palette_commands_with_hints};
use crate::keybindings::Keymap;
use crate::state::CommandPaletteState;

/// Reducer for command palette state.
///
/// Accepts only CommandPaletteAction, making it type-safe and focused.
/// This is a pure state transformation - no side effects or dispatching.
pub fn reduce_command_palette(
    mut state: CommandPaletteState,
    action: &CommandPaletteAction,
    keymap: &Keymap,
) -> CommandPaletteState {
    match action {
        CommandPaletteAction::Char(c) => {
            state.query.push(*c);
            state.selected_index = 0;
        }

        CommandPaletteAction::Backspace => {
            state.query.pop();
            state.selected_index = 0;
        }

        CommandPaletteAction::Clear => {
            // Clear query but keep palette open
            state.query.clear();
            state.selected_index = 0;
        }

        CommandPaletteAction::Close => {
            state.query.clear();
            state.selected_index = 0;
        }

        CommandPaletteAction::Execute => {
            // Just reset state - the middleware handles dispatching the command
            state.query.clear();
            state.selected_index = 0;
        }

        CommandPaletteAction::NavigateNext => {
            let all_commands = get_palette_commands_with_hints(keymap);
            let filtered = filter_commands(&all_commands, &state.query);
            if !filtered.is_empty() {
                state.selected_index = (state.selected_index + 1).min(filtered.len() - 1);
            }
        }

        CommandPaletteAction::NavigatePrev => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }
    }

    state
}
