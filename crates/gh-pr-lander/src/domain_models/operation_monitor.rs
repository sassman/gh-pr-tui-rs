//! Operation Monitor model
//!
//! Types for monitoring PR operations like rebase and merge.

use std::time::Instant;

/// Type of operation being monitored
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// Rebase operation
    Rebase,
    /// Merge operation
    Merge,
}

#[allow(dead_code)]
impl OperationType {
    /// Get the display label for this operation type
    pub fn label(&self) -> &'static str {
        match self {
            OperationType::Rebase => "Rebasing",
            OperationType::Merge => "Merging",
        }
    }
}

/// Represents a PR operation being monitored
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OperationMonitor {
    /// PR number
    pub pr_number: usize,
    /// Type of operation
    pub operation: OperationType,
    /// When the operation started
    pub started_at: Instant,
    /// Number of times we've checked this operation
    pub check_count: usize,
    /// Track SHA to detect rebase completion
    pub last_head_sha: Option<String>,
}

#[allow(dead_code)]
impl OperationMonitor {
    /// Create a new operation monitor for a rebase
    pub fn rebase(pr_number: usize, head_sha: Option<String>) -> Self {
        Self {
            pr_number,
            operation: OperationType::Rebase,
            started_at: Instant::now(),
            check_count: 0,
            last_head_sha: head_sha,
        }
    }

    /// Create a new operation monitor for a merge
    pub fn merge(pr_number: usize) -> Self {
        Self {
            pr_number,
            operation: OperationType::Merge,
            started_at: Instant::now(),
            check_count: 0,
            last_head_sha: None,
        }
    }

    /// Increment the check count
    pub fn increment_check(&mut self) {
        self.check_count += 1;
    }

    /// Update the last head SHA
    pub fn update_head_sha(&mut self, sha: String) {
        self.last_head_sha = Some(sha);
    }

    /// Get the elapsed time since this operation started
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Check if the SHA has changed (indicating rebase completion)
    pub fn sha_changed(&self, new_sha: &str) -> bool {
        self.last_head_sha
            .as_ref()
            .is_some_and(|sha| sha != new_sha)
    }
}
