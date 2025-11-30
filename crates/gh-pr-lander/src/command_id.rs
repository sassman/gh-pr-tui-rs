//! Command identifiers
//!
//! This module defines all command IDs as an enum for type-safe,
//! memory-efficient command references that can be serialized/deserialized.

use serde::{Deserialize, Serialize};

/// Unique identifier for each command in the application.
///
/// Commands are the semantic actions users can trigger. Each command
/// has a unique ID that can be referenced in keybindings and the command palette.
///
/// The enum is serialized as snake_case (e.g., `RepositoryAdd` -> `"repository_add"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandId {
    // === Repository management ===
    /// Add a new repository to track
    RepositoryAdd,
    /// Switch to the next repository
    RepositoryNext,
    /// Switch to the previous repository
    RepositoryPrevious,

    // === Navigation ===
    /// Navigate to the next item (down)
    NavigateNext,
    /// Navigate to the previous item (up)
    NavigatePrevious,
    /// Navigate left
    NavigateLeft,
    /// Navigate right
    NavigateRight,

    // === Scrolling ===
    /// Scroll to the top (gg in vim)
    ScrollToTop,
    /// Scroll to the bottom (G in vim)
    ScrollToBottom,
    /// Scroll down one page
    ScrollPageDown,
    /// Scroll up one page
    ScrollPageUp,
    /// Scroll down half a page (Ctrl+d in vim)
    ScrollHalfPageDown,
    /// Scroll up half a page (Ctrl+u in vim)
    ScrollHalfPageUp,

    // === Debug ===
    /// Toggle the debug console visibility
    DebugToggleConsole,
    /// Clear the debug console logs
    DebugClearLogs,
    /// Dumps the debug logs to file
    DebugLogDump,

    // === Command palette ===
    /// Open the command palette
    CommandPaletteOpen,

    // === PR Selection ===
    /// Toggle selection of current PR
    PrToggleSelection,
    /// Select all PRs in current repository
    PrSelectAll,
    /// Deselect all PRs
    PrDeselectAll,
    /// Refresh PRs for current repository
    PrRefresh,

    // === PR Operations ===
    /// Open current PR in browser
    PrOpenInBrowser,
    /// Merge selected PRs
    PrMerge,
    /// Rebase/update selected PRs
    PrRebase,
    /// Approve selected PRs
    PrApprove,
    /// Close selected PRs without merging
    PrClose,

    // === CI/Build Status ===
    /// Rerun failed CI jobs for current PR
    PrRerunFailedJobs,
    /// Open CI build logs in browser
    PrOpenBuildLogs,

    // === IDE Integration ===
    /// Open current PR in configured IDE
    PrOpenInIDE,

    // === General ===
    /// Close the current view/panel
    GlobalClose,
    /// Quit the application
    GlobalQuit,
}

impl CommandId {
    /// Convert this command ID to an Action
    ///
    /// Some commands require context (like views) to create their action,
    /// those are handled separately in the reducer.
    pub fn to_action(self) -> crate::actions::Action {
        use crate::actions::Action;
        use crate::views::{CommandPaletteView, DebugConsoleView};

        match self {
            // Repository
            Self::RepositoryAdd => Action::RepositoryAdd,
            Self::RepositoryNext => Action::RepositoryNext,
            Self::RepositoryPrevious => Action::RepositoryPrevious,

            // Navigation
            Self::NavigateNext => Action::NavigateNext,
            Self::NavigatePrevious => Action::NavigatePrevious,
            Self::NavigateLeft => Action::NavigateLeft,
            Self::NavigateRight => Action::NavigateRight,

            // Scrolling
            Self::ScrollToTop => Action::ScrollToTop,
            Self::ScrollToBottom => Action::ScrollToBottom,
            Self::ScrollPageDown => Action::ScrollPageDown,
            Self::ScrollPageUp => Action::ScrollPageUp,
            Self::ScrollHalfPageDown => Action::ScrollHalfPageDown,
            Self::ScrollHalfPageUp => Action::ScrollHalfPageUp,

            // Debug
            Self::DebugToggleConsole => Action::PushView(Box::new(DebugConsoleView::new())),
            Self::DebugClearLogs => Action::DebugConsoleClear,
            Self::DebugLogDump => Action::DebugConsoleDumpLogs,

            // Command palette
            Self::CommandPaletteOpen => Action::PushView(Box::new(CommandPaletteView::new())),

            // PR Selection
            Self::PrToggleSelection => Action::PrToggleSelection,
            Self::PrSelectAll => Action::PrSelectAll,
            Self::PrDeselectAll => Action::PrDeselectAll,
            Self::PrRefresh => Action::PrRefresh,

            // PR Operations
            Self::PrOpenInBrowser => Action::PrOpenInBrowser,
            Self::PrMerge => Action::PrMergeRequest,
            Self::PrRebase => Action::PrRebaseRequest,
            Self::PrApprove => Action::PrApproveRequest,
            Self::PrClose => Action::PrCloseRequest,

            // CI/Build Status
            Self::PrRerunFailedJobs => Action::PrRerunFailedJobs,
            Self::PrOpenBuildLogs => Action::PrOpenBuildLogs,

            // IDE Integration
            Self::PrOpenInIDE => Action::PrOpenInIDE,

            // General
            Self::GlobalClose => Action::GlobalClose,
            Self::GlobalQuit => Action::GlobalQuit,
        }
    }

    /// Get the default title for this command (used in command palette)
    pub fn title(&self) -> &'static str {
        match self {
            // Repository
            Self::RepositoryAdd => "Add repository",
            Self::RepositoryNext => "Next repository",
            Self::RepositoryPrevious => "Previous repository",

            // Navigation
            Self::NavigateNext => "Navigate down",
            Self::NavigatePrevious => "Navigate up",
            Self::NavigateLeft => "Navigate left",
            Self::NavigateRight => "Navigate right",

            // Scrolling
            Self::ScrollToTop => "Scroll to top",
            Self::ScrollToBottom => "Scroll to bottom",
            Self::ScrollPageDown => "Page down",
            Self::ScrollPageUp => "Page up",
            Self::ScrollHalfPageDown => "Half page down",
            Self::ScrollHalfPageUp => "Half page up",

            // Debug
            Self::DebugToggleConsole => "Toggle debug console",
            Self::DebugClearLogs => "Clear debug logs",
            Self::DebugLogDump => "Dump debug logs to file",

            // Command palette
            Self::CommandPaletteOpen => "Open command palette",

            // PR Selection
            Self::PrToggleSelection => "Toggle PR selection",
            Self::PrSelectAll => "Select all PRs",
            Self::PrDeselectAll => "Deselect all PRs",
            Self::PrRefresh => "Refresh PRs",

            // PR Operations
            Self::PrOpenInBrowser => "Open PR in browser",
            Self::PrMerge => "Merge PRs",
            Self::PrRebase => "Rebase PRs",
            Self::PrApprove => "Approve PRs",
            Self::PrClose => "Close PRs",

            // CI/Build Status
            Self::PrRerunFailedJobs => "Rerun failed CI jobs",
            Self::PrOpenBuildLogs => "Open CI build logs",

            // IDE Integration
            Self::PrOpenInIDE => "Open PR diff in IDE",

            // General
            Self::GlobalClose => "Close",
            Self::GlobalQuit => "Quit",
        }
    }

    /// Get the default description for this command
    pub fn description(&self) -> &'static str {
        match self {
            // Repository
            Self::RepositoryAdd => "Add a new repository to track",
            Self::RepositoryNext => "Switch to the next repository",
            Self::RepositoryPrevious => "Switch to the previous repository",

            // Navigation
            Self::NavigateNext => "Move selection down",
            Self::NavigatePrevious => "Move selection up",
            Self::NavigateLeft => "Move selection or scroll left",
            Self::NavigateRight => "Move selection or scroll right",

            // Scrolling
            Self::ScrollToTop => "Jump to the first item",
            Self::ScrollToBottom => "Jump to the last item",
            Self::ScrollPageDown => "Scroll down by one page",
            Self::ScrollPageUp => "Scroll up by one page",
            Self::ScrollHalfPageDown => "Scroll down by half a page",
            Self::ScrollHalfPageUp => "Scroll up by half a page",

            // Debug
            Self::DebugToggleConsole => "Show or hide the debug console",
            Self::DebugClearLogs => "Clear all debug console logs",
            Self::DebugLogDump => "Save debug logs to a file",

            // Command palette
            Self::CommandPaletteOpen => "Open the command palette to search and execute commands",

            // PR Selection
            Self::PrToggleSelection => "Toggle selection of the current PR for bulk operations",
            Self::PrSelectAll => "Select all PRs in the current repository",
            Self::PrDeselectAll => "Clear all PR selections",
            Self::PrRefresh => "Refresh PRs for the current repository",

            // PR Operations
            Self::PrOpenInBrowser => "Open the current PR in your default web browser",
            Self::PrMerge => "Merge selected PRs (or current PR if none selected)",
            Self::PrRebase => "Update selected PRs with latest from base branch",
            Self::PrApprove => "Approve selected PRs with a review",
            Self::PrClose => "Close selected PRs without merging",

            // CI/Build Status
            Self::PrRerunFailedJobs => "Rerun failed CI workflow jobs for the current PR",
            Self::PrOpenBuildLogs => "Open CI build logs in your default web browser",

            // IDE Integration
            Self::PrOpenInIDE => "Open the PR diff in your configured IDE (uses gh pr view)",

            // General
            Self::GlobalClose => "Close the current view or panel",
            Self::GlobalQuit => "Exit the application",
        }
    }

    /// Get the category for this command (used for grouping in command palette)
    pub fn category(&self) -> &'static str {
        match self {
            Self::RepositoryAdd | Self::RepositoryNext | Self::RepositoryPrevious => "Repository",

            Self::NavigateNext
            | Self::NavigatePrevious
            | Self::NavigateLeft
            | Self::NavigateRight => "Navigation",

            Self::ScrollToTop
            | Self::ScrollToBottom
            | Self::ScrollPageDown
            | Self::ScrollPageUp
            | Self::ScrollHalfPageDown
            | Self::ScrollHalfPageUp => "Scroll",

            Self::DebugToggleConsole | Self::DebugClearLogs | Self::DebugLogDump => "Debug",

            Self::CommandPaletteOpen => "Command Palette",

            Self::PrToggleSelection
            | Self::PrSelectAll
            | Self::PrDeselectAll
            | Self::PrRefresh
            | Self::PrOpenInBrowser
            | Self::PrMerge
            | Self::PrRebase
            | Self::PrApprove
            | Self::PrClose
            | Self::PrRerunFailedJobs
            | Self::PrOpenBuildLogs
            | Self::PrOpenInIDE => "Pull Request",

            Self::GlobalClose | Self::GlobalQuit => "General",
        }
    }

    /// Check if this command should appear in the command palette
    pub fn show_in_palette(&self) -> bool {
        match self {
            // Navigation/scroll commands are typically not shown in palette
            // (they're keyboard-driven)
            Self::NavigateNext
            | Self::NavigatePrevious
            | Self::NavigateLeft
            | Self::NavigateRight
            | Self::ScrollToTop
            | Self::ScrollToBottom
            | Self::ScrollPageDown
            | Self::ScrollPageUp
            | Self::ScrollHalfPageDown
            | Self::ScrollHalfPageUp
            | Self::CommandPaletteOpen => false,

            // All others are shown
            _ => true,
        }
    }
}
