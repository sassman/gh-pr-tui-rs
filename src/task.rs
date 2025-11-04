/// Background task system for handling heavy operations without blocking UI

use crate::{log::PrContext, pr::Pr, state::Repo, PrFilter};
use octocrab::Octocrab;

/// Background tasks that can be executed asynchronously
#[derive(Debug)]
pub enum BackgroundTask {
    LoadAllRepos {
        repos: Vec<Repo>,
        filter: PrFilter,
        octocrab: Octocrab,
    },
    LoadSingleRepo {
        repo_index: usize,
        repo: Repo,
        filter: PrFilter,
        octocrab: Octocrab,
    },
    CheckMergeStatus {
        repo_index: usize,
        repo: Repo,
        pr_numbers: Vec<usize>,
        octocrab: Octocrab,
    },
    Rebase {
        repo: Repo,
        prs: Vec<Pr>,
        selected_indices: Vec<usize>,
        octocrab: Octocrab,
    },
    Merge {
        repo: Repo,
        prs: Vec<Pr>,
        selected_indices: Vec<usize>,
        octocrab: Octocrab,
    },
    RerunFailedJobs {
        repo: Repo,
        pr_numbers: Vec<usize>,
        octocrab: Octocrab,
    },
    FetchBuildLogs {
        repo: Repo,
        pr_number: usize,
        head_sha: String,
        octocrab: Octocrab,
        pr_context: PrContext,
    },
    OpenPRInIDE {
        repo: Repo,
        pr_number: usize,
        ide_command: String,
        temp_dir: String,
    },
    /// Poll a PR to check if it's actually merged (for merge bot)
    PollPRMergeStatus {
        repo_index: usize,
        repo: Repo,
        pr_number: usize,
        octocrab: Octocrab,
        is_checking_ci: bool, // If true, use longer sleep (15s) for CI checks
    },
}
