//! Command list for command palette
//!
//! This module defines all available commands that can be executed via the command palette.
//! Commands are simple structs that map user-facing descriptions to actions.

use crate::actions::Action;

/// A single command that can be executed
#[derive(Debug, Clone)]
pub struct Command {
    pub title: String,
    pub description: String,
    pub category: String,
    pub shortcut_hint: Option<String>,
    pub action: Action,
}

/// Get all available commands
///
/// Returns a list of all commands that can be executed in the application.
/// Commands are organized by category for better discoverability.
pub fn get_all_commands() -> Vec<Command> {
    vec![
        // Debug category
        Command {
            title: "Toggle debug console".to_string(),
            description: "Show or hide the debug console".to_string(),
            category: "Debug".to_string(),
            shortcut_hint: Some("`".to_string()),
            action: Action::PushView(Box::new(crate::views::DebugConsoleView::new())),
        },
        Command {
            title: "Clear debug logs".to_string(),
            description: "Clear all debug console logs".to_string(),
            category: "Debug".to_string(),
            shortcut_hint: None,
            action: Action::DebugConsoleClear,
        },
        // Navigation category
        Command {
            title: "Next repository".to_string(),
            description: "Switch to the next repository".to_string(),
            category: "Navigation".to_string(),
            shortcut_hint: Some("Tab".to_string()),
            action: Action::RepositoryNext,
        },
        Command {
            title: "Previous repository".to_string(),
            description: "Switch to the previous repository".to_string(),
            category: "Navigation".to_string(),
            shortcut_hint: Some("Shift+Tab".to_string()),
            action: Action::RepositoryPrevious,
        },
        // General category
        Command {
            title: "Quit application".to_string(),
            description: "Exit the application".to_string(),
            category: "General".to_string(),
            shortcut_hint: Some("q / Ctrl+C".to_string()),
            action: Action::GlobalQuit,
        },
        Command {
            title: "Close current view".to_string(),
            description: "Close the current view or panel".to_string(),
            category: "General".to_string(),
            shortcut_hint: Some("Esc / q".to_string()),
            action: Action::GlobalClose,
        },
        // Repository management
        Command {
            title: "Add repository".to_string(),
            description: "Add a new repository to track".to_string(),
            category: "Repository".to_string(),
            shortcut_hint: Some("p → a".to_string()),
            action: Action::RepositoryAdd,
        },
    ]
}

/// Filter commands based on a search query
///
/// Performs case-insensitive fuzzy matching on title and description.
pub fn filter_commands(commands: &[Command], query: &str) -> Vec<Command> {
    if query.is_empty() {
        return commands.to_vec();
    }

    let query_lower = query.to_lowercase();
    commands
        .iter()
        .filter(|cmd| {
            cmd.title.to_lowercase().contains(&query_lower)
                || cmd.description.to_lowercase().contains(&query_lower)
                || cmd.category.to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect()
}
