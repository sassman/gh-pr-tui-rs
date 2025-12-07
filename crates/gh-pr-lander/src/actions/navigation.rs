//! Navigation actions - shared across multiple screens
//!
//! These are generic navigation actions that views can translate
//! into their screen-specific actions.

/// Generic navigation actions (vim-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationAction {
    /// Navigate to next item (j, down arrow)
    Next,
    /// Navigate to previous item (k, up arrow)
    Previous,
    /// Navigate left (h, left arrow)
    Left,
    /// Navigate right (l, right arrow)
    Right,
    /// Navigate to top (gg)
    ToTop,
    /// Navigate to bottom (G)
    ToBottom,
}
