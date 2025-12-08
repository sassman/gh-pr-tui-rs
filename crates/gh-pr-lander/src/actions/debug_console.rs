//! Debug Console screen actions
//!
//! Actions specific to the debug console overlay.

use std::fmt;

/// Actions for the Debug Console screen
#[derive(Clone)]
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

// Custom Debug to avoid logging full line contents (prevents feedback loop)
impl fmt::Debug for DebugConsoleAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NavigateNext => write!(f, "NavigateNext"),
            Self::NavigatePrevious => write!(f, "NavigatePrevious"),
            Self::NavigateToTop => write!(f, "NavigateToTop"),
            Self::NavigateToBottom => write!(f, "NavigateToBottom"),
            Self::Clear => write!(f, "Clear"),
            Self::SetVisibleHeight(h) => write!(f, "SetVisibleHeight({})", h),
            Self::LinesUpdated(lines) => write!(f, "LinesUpdated(<{} lines>)", lines.len()),
        }
    }
}
