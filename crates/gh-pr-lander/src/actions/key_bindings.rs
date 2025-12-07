//! Key Bindings screen actions
//!
//! Actions specific to the key bindings help overlay.

/// Actions for the Key Bindings screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyBindingsAction {
    // Navigation (translated from NavigationAction)
    /// Scroll to next item
    NavigateNext,
    /// Scroll to previous item
    NavigatePrevious,
    /// Scroll to top
    NavigateToTop,
    /// Scroll to bottom
    NavigateToBottom,

    // Specific actions
    /// Close the key bindings panel
    Close,
}
