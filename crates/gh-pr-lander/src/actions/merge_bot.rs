//! Merge Bot Actions
//!
//! Actions for the automated merge bot that manages PR merge queues.

/// Actions for the merge bot subsystem
#[derive(Debug, Clone)]
pub enum MergeBotAction {
    /// Start the merge bot
    Start,
    /// Stop the merge bot
    Stop,
    /// Add selected PRs to the merge queue
    AddToQueue,
    /// Remove a PR from the merge queue
    RemoveFromQueue(usize, usize),
    /// Periodic tick for merge bot processing
    Tick,
    /// A PR check has completed (repo_idx, pr_number, success)
    CheckComplete(usize, usize, bool),
    /// A PR merge has completed (repo_idx, pr_number, success, message)
    MergeComplete(usize, usize, bool, String),
}
