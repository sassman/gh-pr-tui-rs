//! Command registry
//!
//! This module defines commands that can be executed via the command palette
//! or keyboard shortcuts. Commands wrap CommandIds with display metadata.

use crate::command_id::CommandId;
use crate::keybindings::Keymap;

/// A command that can be executed via command palette or keybinding
#[derive(Debug, Clone)]
pub struct Command {
    /// The unique identifier for this command
    pub id: CommandId,
    /// The keyboard shortcut hint (populated from keybindings)
    pub shortcut_hint: Option<String>,
}

impl Command {
    /// Create a new command from a CommandId
    pub fn new(id: CommandId) -> Self {
        Self {
            id,
            shortcut_hint: None,
        }
    }

    /// Create a command with a shortcut hint
    pub fn with_shortcut(id: CommandId, hint: impl Into<String>) -> Self {
        Self {
            id,
            shortcut_hint: Some(hint.into()),
        }
    }

    /// Get the title for display
    pub fn title(&self) -> &'static str {
        self.id.title()
    }

    /// Get the description for display
    pub fn description(&self) -> &'static str {
        self.id.description()
    }

    /// Get the category for grouping
    pub fn category(&self) -> &'static str {
        self.id.category()
    }
}

/// Get all command IDs that should appear in the command palette
fn palette_command_ids() -> Vec<CommandId> {
    use CommandId::*;

    vec![
        RepositoryAdd,
        RepositoryNext,
        RepositoryPrevious,
        DebugToggleConsoleView,
        DebugClearLogs,
        CommandPaletteOpen,
        GlobalClose,
        GlobalQuit,
    ]
    .into_iter()
    .filter(|id| id.show_in_palette())
    .collect()
}

/// Get all commands with shortcut hints populated from the keymap
///
/// Uses `compact_hint_for_command` to show all keybindings for a command
/// (e.g., "q/Esc" for GlobalClose instead of just "q")
pub fn get_palette_commands_with_hints(keymap: &Keymap) -> Vec<Command> {
    palette_command_ids()
        .into_iter()
        .map(|id| {
            if let Some(hint) = keymap.compact_hint_for_command(id) {
                Command::with_shortcut(id, hint)
            } else {
                Command::new(id)
            }
        })
        .collect()
}

/// Filter commands based on a search query
///
/// Performs case-insensitive fuzzy matching on title, description, and category.
pub fn filter_commands(commands: &[Command], query: &str) -> Vec<Command> {
    if query.is_empty() {
        return commands.to_vec();
    }

    let query_lower = query.to_lowercase();
    commands
        .iter()
        .filter(|cmd| {
            cmd.title().to_lowercase().contains(&query_lower)
                || cmd.description().to_lowercase().contains(&query_lower)
                || cmd.category().to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect()
}
