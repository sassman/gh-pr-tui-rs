//! Command Palette screen actions
//!
//! Actions specific to the command palette overlay.

/// Actions for the Command Palette screen
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPaletteAction {
    // Navigation (translated from NavigationAction)
    /// Navigate to next command in the list
    NavigateNext,
    /// Navigate to previous command in the list
    NavigatePrev,

    // Text input (translated from TextInputAction)
    /// Character typed into search field
    Char(char),
    /// Backspace pressed in search field
    Backspace,
    /// Clear entire query
    Clear,

    // Specific actions
    /// Close the command palette
    Close,
    /// Execute selected command
    Execute,
}
