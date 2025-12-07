//! Context-sensitive actions
//!
//! These are semantic actions that views interpret differently based on context.
//! For example, `Confirm` (Enter key) means:
//! - PR table: Open PR in browser
//! - Command palette: Execute selected command
//! - Add repository: Submit form
//! - Build log: Toggle section expansion

/// Semantic actions that views interpret differently.
///
/// These represent user intent, not specific operations.
/// Each view translates them to view-specific actions via `translate_context_action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAction {
    /// Primary action on focused item (Enter key)
    ///
    /// Views interpret this as their primary action:
    /// - PR table: Open in browser
    /// - Command palette: Execute selected command
    /// - Add repository: Submit form
    /// - Build log: Toggle section expansion
    Confirm,

    /// Toggle state of focused item (Space key)
    ///
    /// Views interpret this as toggling selection/state:
    /// - PR table: Toggle PR selection
    /// - Build log: Toggle section expansion
    ToggleSelect,

    /// Select all items (Ctrl+A)
    SelectAll,

    /// Deselect all items
    DeselectAll,
}
