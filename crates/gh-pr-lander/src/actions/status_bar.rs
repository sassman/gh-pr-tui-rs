//! Status Bar Actions
//!
//! Actions for the status bar - user feedback for operations.

use crate::state::StatusKind;

/// Actions for the status bar
#[derive(Debug, Clone)]
pub enum StatusBarAction {
    /// Push a new status message
    Push {
        kind: StatusKind,
        message: String,
        source: String,
    },
    /// Clear all status messages
    Clear,
}

impl StatusBarAction {
    /// Create a running status action
    pub fn running(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self::Push {
            kind: StatusKind::Running,
            message: message.into(),
            source: source.into(),
        }
    }

    /// Create a success status action
    pub fn success(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self::Push {
            kind: StatusKind::Success,
            message: message.into(),
            source: source.into(),
        }
    }

    /// Create an error status action
    pub fn error(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self::Push {
            kind: StatusKind::Error,
            message: message.into(),
            source: source.into(),
        }
    }

    /// Create a warning status action
    pub fn warning(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self::Push {
            kind: StatusKind::Warning,
            message: message.into(),
            source: source.into(),
        }
    }

    /// Create an info status action
    pub fn info(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self::Push {
            kind: StatusKind::Info,
            message: message.into(),
            source: source.into(),
        }
    }
}
