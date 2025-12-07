//! Auto Merge model
//!
//! Types for managing PRs in the auto-merge queue.

use std::time::Instant;

/// Represents a PR in the auto-merge queue
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AutoMergePr {
    /// PR number
    pub pr_number: usize,
    /// When the PR was added to the queue
    pub started_at: Instant,
    /// Number of times we've checked this PR
    pub check_count: usize,
}

#[allow(dead_code)]
impl AutoMergePr {
    /// Create a new auto-merge PR entry
    pub fn new(pr_number: usize) -> Self {
        Self {
            pr_number,
            started_at: Instant::now(),
            check_count: 0,
        }
    }

    /// Increment the check count
    pub fn increment_check(&mut self) {
        self.check_count += 1;
    }

    /// Get the elapsed time since this PR was queued
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}
