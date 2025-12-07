//! Add Repository screen actions
//!
//! Actions specific to the add repository form overlay.

/// Actions for the Add Repository form screen
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddRepositoryAction {
    // Navigation (translated from NavigationAction)
    /// Move to next field (Tab)
    NextField,
    /// Move to previous field (Shift+Tab)
    PrevField,

    // Text input (translated from TextInputAction)
    /// Character typed into current field
    Char(char),
    /// Backspace pressed in current field
    Backspace,
    /// Clear entire current field
    ClearField,

    // Specific actions
    /// Confirm and add the repository (Enter)
    Confirm,
    /// Close the form without adding (Esc)
    Close,
}
