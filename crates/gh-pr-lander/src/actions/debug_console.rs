//! Debug Console screen actions
//!
//! Actions specific to the debug console overlay.

/// Actions for the Debug Console screen
#[derive(Debug, Clone)]
pub enum DebugConsoleAction {
    // Navigation (translated from NavigationAction)
    /// Scroll to next log entry
    NavigateNext,
    /// Scroll to previous log entry
    NavigatePrevious,
    /// Scroll to top (oldest logs)
    NavigateToTop,
    /// Scroll to bottom (newest logs)
    NavigateToBottom,

    // Specific actions
    /// Clear all logs from view
    Clear,
    /// Update visible height (for proper scroll bounds)
    SetVisibleHeight(usize),
    /// Batch update of lines from middleware
    LinesUpdated(Vec<String>),
}
