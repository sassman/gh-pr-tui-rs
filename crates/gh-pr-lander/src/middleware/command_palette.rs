//! Command Palette Middleware
//!
//! This middleware has two responsibilities:
//! 1. Translate generic TextInput actions to CommandPalette-specific actions
//!    when the command palette is active
//! 2. Execute the selected command when CommandPaletteExecute is dispatched

use crate::actions::Action;
use crate::commands::{filter_commands, get_palette_commands_with_hints};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::ViewId;

/// Middleware that handles command palette interactions
pub struct CommandPaletteMiddleware;

impl CommandPaletteMiddleware {
    pub fn new() -> Self {
        Self
    }

    /// Check if the command palette is the active view
    fn is_active(state: &AppState) -> bool {
        state.active_view().view_id() == ViewId::CommandPalette
    }
}

impl Default for CommandPaletteMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for CommandPaletteMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Only process when command palette is active
        if !Self::is_active(state) {
            return true; // Pass through
        }

        match action {
            // Translate generic TextInput actions to CommandPalette-specific actions
            Action::TextInputChar(c) => {
                dispatcher.dispatch(Action::CommandPaletteChar(*c));
                false // Consume the original action
            }

            Action::TextInputBackspace => {
                dispatcher.dispatch(Action::CommandPaletteBackspace);
                false
            }

            Action::TextInputEscape => {
                // Escape behavior: clear query if not empty, otherwise close
                if !state.command_palette.query.is_empty() {
                    // Dispatch backspace for each character to clear
                    // Or we could add a CommandPaletteClear action
                    dispatcher.dispatch(Action::CommandPaletteClose);
                } else {
                    dispatcher.dispatch(Action::CommandPaletteClose);
                }
                false
            }

            Action::TextInputConfirm => {
                // First dispatch the execute action (middleware will handle command dispatch)
                // Then the reducer will close the palette
                dispatcher.dispatch(Action::CommandPaletteExecute);
                false
            }

            Action::NavigateNext => {
                dispatcher.dispatch(Action::CommandPaletteNavigateNext);
                false
            }

            Action::NavigatePrevious => {
                dispatcher.dispatch(Action::CommandPaletteNavigatePrev);
                false
            }

            // Handle command execution - dispatch the selected command's action
            Action::CommandPaletteExecute => {
                let all_commands = get_palette_commands_with_hints(&state.keymap);
                let filtered = filter_commands(&all_commands, &state.command_palette.query);

                if let Some(cmd) = filtered.get(state.command_palette.selected_index) {
                    log::debug!("Command palette executing: {}", cmd.title());
                    dispatcher.dispatch(cmd.id.to_action());
                }
                // Let the action continue to the reducer to close the palette
                true
            }

            // All other actions pass through
            _ => true,
        }
    }
}
