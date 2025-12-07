//! Command Palette Middleware
//!
//! Executes the selected command when CommandPalette::Execute is dispatched.
//! Text input and navigation are handled via view translation (translate_text_input/translate_navigation).

use crate::actions::{Action, CommandPaletteAction};
use crate::commands::{filter_commands, get_palette_commands_with_hints};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// Middleware that handles command palette command execution
pub struct CommandPaletteMiddleware;

impl CommandPaletteMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CommandPaletteMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for CommandPaletteMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Handle command execution - dispatch the selected command's action
        if let Action::CommandPalette(CommandPaletteAction::Execute) = action {
            let all_commands = get_palette_commands_with_hints(&state.keymap);
            let filtered = filter_commands(&all_commands, &state.command_palette.query);

            if let Some(cmd) = filtered.get(state.command_palette.selected_index) {
                log::debug!("Command palette executing: {}", cmd.title());
                dispatcher.dispatch(cmd.id.to_action());
            }
            // Let the action continue to the reducer to close the palette
            return true;
        }

        // All other actions pass through
        true
    }
}
