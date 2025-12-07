//! Task Status model
//!
//! Status tracking for background tasks.

/// Status of a background task
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TaskStatus {
    /// Status message
    pub message: String,
    /// Type of status
    pub status_type: TaskStatusType,
}

#[allow(dead_code)]
impl TaskStatus {
    /// Create a new running task status
    pub fn running(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_type: TaskStatusType::Running,
        }
    }

    /// Create a new success task status
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_type: TaskStatusType::Success,
        }
    }

    /// Create a new error task status
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_type: TaskStatusType::Error,
        }
    }

    /// Create a new warning task status
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_type: TaskStatusType::Warning,
        }
    }
}

/// Type of task status
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatusType {
    /// Task is running
    Running,
    /// Task completed successfully
    Success,
    /// Task failed with an error
    Error,
    /// Task completed with warnings
    Warning,
}
