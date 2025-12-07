//! Text input actions - shared across screens with text input capability
//!
//! These are generic text input actions that views can translate
//! into their screen-specific actions.

/// Generic text input actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputAction {
    /// Character typed into input field
    Char(char),
    /// Backspace pressed - remove last character
    Backspace,
    /// Clear entire line (Cmd+Backspace or Ctrl+U)
    ClearLine,
    /// Escape pressed - typically closes or clears
    Escape,
    /// Enter pressed - confirm/execute
    Confirm,
}
