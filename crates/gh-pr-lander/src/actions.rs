use ratatui::crossterm::event::KeyEvent;

use crate::state::ActiveView;

/// Actions represent all possible state changes in the application.
/// Actions are prefixed by scope to indicate which part of the app they affect.
#[derive(Debug, Clone)]
pub enum Action {
    // Global actions (not tied to any specific view)
    GlobalKeyPressed(KeyEvent),
    GlobalClose,
    GlobalQuit,
    GlobalActivateView(ActiveView),

    // Local actions (dispatched to active view for handling)
    LocalKeyPressed(char), // Key pressed in active view context

    // Navigation actions (semantic, vim-style)
    NavNext,      // j, down arrow
    NavPrevious,  // k, up arrow
    NavLeft,      // h, left arrow
    NavRight,     // l, right arrow
    NavJumpToEnd, // G

    // Debug console actions
    DebugConsoleClear,            // Clear debug console logs
    DebugConsoleLogAdded(String), // New log message added

    // No-op action
    None,
}
