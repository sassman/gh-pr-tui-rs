//! Status Bar State

use chrono::{DateTime, Local};
use std::collections::VecDeque;

/// Kind of status message (determines icon and color)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    /// Action started/in progress
    Running,
    /// Action completed successfully
    Success,
    /// Action failed with error
    Error,
    /// Warning (non-fatal issue)
    Warning,
    /// Informational message
    Info,
}

impl StatusKind {
    /// Get the emoji for this status kind
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Running => "⏳",
            Self::Success => "✅",
            Self::Error => "🚨",
            Self::Warning => "⚠️",
            Self::Info => "ℹ️",
        }
    }
}

/// A single status message with metadata
#[derive(Debug, Clone)]
pub struct StatusMessage {
    /// When the message was created
    pub timestamp: DateTime<Local>,
    /// Type of status
    pub kind: StatusKind,
    /// The message text (crisp and concise)
    pub message: String,
    /// The action that triggered this message (for context)
    pub source_action: String,
}

impl StatusMessage {
    /// Create a new status message with current timestamp
    pub fn new(
        kind: StatusKind,
        message: impl Into<String>,
        source_action: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Local::now(),
            kind,
            message: message.into(),
            source_action: source_action.into(),
        }
    }
}

/// Status bar state - history of messages
#[derive(Debug, Clone)]
pub struct StatusBarState {
    /// Message history (newest at back) - VecDeque for O(1) pop_front
    pub messages: VecDeque<StatusMessage>,
    /// Maximum messages to keep (prevent unbounded growth)
    pub max_history: usize,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
            max_history: 100,
        }
    }
}

impl StatusBarState {
    /// Get the latest message (if any)
    pub fn latest(&self) -> Option<&StatusMessage> {
        self.messages.back()
    }

    /// Push a new message, trimming oldest if over limit
    pub fn push(&mut self, message: StatusMessage) {
        self.messages.push_back(message);
        // Trim oldest if over limit - O(1) with VecDeque
        if self.messages.len() > self.max_history {
            self.messages.pop_front();
        }
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}
