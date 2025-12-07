//! Available Action - represents an action that can be performed in the current context.
//!
//! Used for rendering contextual help/suggestions in the UI footer.

use crate::command_id::CommandId;

/// Category for grouping available actions in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCategory {
    /// Primary actions (confirm, open, execute)
    Primary,
    /// Navigation actions (up, down, scroll)
    Navigation,
    /// Selection actions (toggle, select all)
    Selection,
}

/// An action available in the current view context.
///
/// Views report these to show contextual help in the footer.
#[derive(Debug, Clone)]
pub struct AvailableAction {
    /// The command that triggers this action
    pub command: CommandId,
    /// Short label for display (e.g., "Open", "Merge", "Select")
    pub label: &'static str,
    /// Category for grouping/ordering
    pub category: ActionCategory,
}

impl AvailableAction {
    /// Create a new available action.
    pub fn new(command: CommandId, label: &'static str, category: ActionCategory) -> Self {
        Self {
            command,
            label,
            category,
        }
    }

    /// Create a primary action.
    pub fn primary(command: CommandId, label: &'static str) -> Self {
        Self::new(command, label, ActionCategory::Primary)
    }

    /// Create a navigation action.
    pub fn navigation(command: CommandId, label: &'static str) -> Self {
        Self::new(command, label, ActionCategory::Navigation)
    }

    /// Create a selection action.
    pub fn selection(command: CommandId, label: &'static str) -> Self {
        Self::new(command, label, ActionCategory::Selection)
    }
}
