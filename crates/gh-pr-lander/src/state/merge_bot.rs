//! Merge Bot State

/// Merge bot state
#[derive(Debug, Clone, Default)]
pub struct MergeBotState {
    /// Whether the merge bot is actively running
    pub active: bool,
    /// Queue of PRs waiting to be merged
    pub queue: Vec<MergeBotEntry>,
    /// Currently processing entry (if any)
    pub current: Option<MergeBotEntry>,
}

/// An entry in the merge bot queue
#[derive(Debug, Clone)]
pub struct MergeBotEntry {
    pub repo_idx: usize,
    pub pr_number: usize,
    pub status: MergeBotStatus,
    pub added_at: chrono::DateTime<chrono::Local>,
}

/// Status of a PR in the merge bot queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeBotStatus {
    /// Waiting in queue
    #[default]
    Queued,
    /// Checking CI status
    CheckingCI,
    /// Waiting for CI to complete
    WaitingForCI,
    /// Ready to merge (all checks passed)
    ReadyToMerge,
    /// Currently merging
    Merging,
    /// Successfully merged
    Merged,
    /// Failed to merge
    Failed,
}
