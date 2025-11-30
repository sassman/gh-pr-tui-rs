//! Octocrab-based GitHub API client
//!
//! Direct implementation of the `GitHubClient` trait using the octocrab library.
//! This client makes real API calls without any caching.

use crate::client::GitHubClient;
use crate::types::{
    CheckConclusion, CheckRun, CheckRunStatus, CheckState, CheckStatus, CommitStatus,
    MergeableState, PullRequest,
};
use async_trait::async_trait;
use log::debug;
use octocrab::Octocrab;
use std::sync::Arc;

/// Direct GitHub API client using octocrab
///
/// This is the base implementation that makes actual API calls.
/// It can be wrapped by `CachedGitHubClient` to add caching behavior.
#[derive(Debug, Clone)]
pub struct OctocrabClient {
    octocrab: Arc<Octocrab>,
}

impl OctocrabClient {
    /// Create a new client with the given octocrab instance
    pub fn new(octocrab: Arc<Octocrab>) -> Self {
        Self { octocrab }
    }

    /// Get a reference to the underlying octocrab instance
    pub fn octocrab(&self) -> &Octocrab {
        &self.octocrab
    }
}

#[async_trait]
impl GitHubClient for OctocrabClient {
    async fn fetch_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        base_branch: Option<&str>,
    ) -> anyhow::Result<Vec<PullRequest>> {
        debug!("Fetching PRs for {}/{}", owner, repo);

        let mut prs = Vec::new();
        let mut page_num = 1u32;
        const MAX_PRS: usize = 50;
        const PER_PAGE: u8 = 30;

        loop {
            let pulls = self.octocrab.pulls(owner, repo);
            let mut request = pulls
                .list()
                .state(octocrab::params::State::Open)
                .per_page(PER_PAGE)
                .page(page_num);

            if let Some(branch) = base_branch {
                request = request.head(branch);
            }

            let page = request.send().await?;
            let page_is_empty = page.items.is_empty();

            for pr in page.items {
                if prs.len() >= MAX_PRS {
                    break;
                }
                prs.push(convert_pull_request(&pr));
            }

            if prs.len() >= MAX_PRS || page_is_empty {
                break;
            }

            page_num += 1;
        }

        // Sort by PR number (descending) for stable ordering
        prs.sort_by(|a, b| b.number.cmp(&a.number));

        debug!("Fetched {} PRs for {}/{}", prs.len(), owner, repo);
        Ok(prs)
    }

    async fn fetch_check_runs(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> anyhow::Result<Vec<CheckRun>> {
        debug!(
            "Fetching check runs for {}/{} @ {}",
            owner, repo, commit_sha
        );

        let checks = self
            .octocrab
            .checks(owner, repo)
            .list_check_runs_for_git_ref(commit_sha.to_string().into())
            .send()
            .await?;

        let runs = checks
            .check_runs
            .into_iter()
            .map(|run| {
                // Determine status based on whether completed_at is set
                let status = if run.completed_at.is_some() {
                    CheckRunStatus::Completed
                } else if run.started_at.is_some() {
                    CheckRunStatus::InProgress
                } else {
                    CheckRunStatus::Queued
                };

                CheckRun {
                    id: run.id.0,
                    name: run.name,
                    status,
                    conclusion: run.conclusion.as_ref().map(|c| convert_conclusion_string(c)),
                    details_url: run.details_url,
                    started_at: run.started_at,
                    completed_at: run.completed_at,
                }
            })
            .collect();

        Ok(runs)
    }

    async fn fetch_commit_status(
        &self,
        owner: &str,
        repo: &str,
        commit_sha: &str,
    ) -> anyhow::Result<CheckStatus> {
        debug!(
            "Fetching commit status for {}/{} @ {}",
            owner, repo, commit_sha
        );

        // Use raw GET request since octocrab's Reference type doesn't support commit SHAs
        let route = format!("/repos/{}/{}/commits/{}/status", owner, repo, commit_sha);
        let status: octocrab::models::CombinedStatus =
            self.octocrab.get(route, None::<&()>).await?;

        let state = convert_status_state(&status.state);
        let statuses = status
            .statuses
            .into_iter()
            .map(|s| CommitStatus {
                context: s.context.unwrap_or_else(|| "unknown".to_string()),
                state: convert_status_state(&s.state),
                description: s.description,
                target_url: s.target_url,
            })
            .collect();

        Ok(CheckStatus {
            state,
            total_count: status.total_count as u64,
            statuses,
        })
    }
}

/// Convert octocrab PullRequest to our PullRequest type
fn convert_pull_request(pr: &octocrab::models::pulls::PullRequest) -> PullRequest {
    PullRequest {
        number: pr.number,
        title: pr.title.clone().unwrap_or_default(),
        body: pr.body.clone(),
        author: pr
            .user
            .as_ref()
            .map(|u| u.login.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        comments: pr.comments.unwrap_or(0),
        head_sha: pr.head.sha.clone(),
        base_branch: pr.base.ref_field.clone(),
        head_branch: pr.head.ref_field.clone(),
        mergeable: pr.mergeable,
        mergeable_state: pr
            .mergeable_state
            .as_ref()
            .map(convert_mergeable_state),
        created_at: pr.created_at.unwrap_or_else(chrono::Utc::now),
        updated_at: pr.updated_at.unwrap_or_else(chrono::Utc::now),
        html_url: pr
            .html_url
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default(),
    }
}

/// Convert octocrab MergeableState enum to our enum
fn convert_mergeable_state(state: &octocrab::models::pulls::MergeableState) -> MergeableState {
    use octocrab::models::pulls::MergeableState as OMS;
    match state {
        OMS::Clean => MergeableState::Clean,
        OMS::Behind => MergeableState::Behind,
        OMS::Dirty => MergeableState::Dirty,
        OMS::Blocked => MergeableState::Blocked,
        OMS::Unstable => MergeableState::Unstable,
        OMS::Unknown => MergeableState::Unknown,
        _ => MergeableState::Unknown,
    }
}

/// Convert conclusion string from GitHub API to our enum
fn convert_conclusion_string(conclusion: &str) -> CheckConclusion {
    match conclusion.to_lowercase().as_str() {
        "success" => CheckConclusion::Success,
        "failure" => CheckConclusion::Failure,
        "neutral" => CheckConclusion::Neutral,
        "cancelled" => CheckConclusion::Cancelled,
        "skipped" => CheckConclusion::Skipped,
        "timed_out" => CheckConclusion::TimedOut,
        "action_required" => CheckConclusion::ActionRequired,
        "stale" => CheckConclusion::Stale,
        _ => CheckConclusion::Neutral,
    }
}

/// Convert octocrab StatusState to our CheckState
fn convert_status_state(state: &octocrab::models::StatusState) -> CheckState {
    match state {
        octocrab::models::StatusState::Success => CheckState::Success,
        octocrab::models::StatusState::Pending => CheckState::Pending,
        octocrab::models::StatusState::Failure => CheckState::Failure,
        octocrab::models::StatusState::Error => CheckState::Error,
        _ => CheckState::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_conclusion_string() {
        assert_eq!(convert_conclusion_string("success"), CheckConclusion::Success);
        assert_eq!(convert_conclusion_string("SUCCESS"), CheckConclusion::Success);
        assert_eq!(convert_conclusion_string("failure"), CheckConclusion::Failure);
        assert_eq!(convert_conclusion_string("neutral"), CheckConclusion::Neutral);
        assert_eq!(convert_conclusion_string("cancelled"), CheckConclusion::Cancelled);
        assert_eq!(convert_conclusion_string("skipped"), CheckConclusion::Skipped);
        assert_eq!(convert_conclusion_string("timed_out"), CheckConclusion::TimedOut);
        assert_eq!(
            convert_conclusion_string("action_required"),
            CheckConclusion::ActionRequired
        );
        assert_eq!(convert_conclusion_string("stale"), CheckConclusion::Stale);
        assert_eq!(convert_conclusion_string("unknown"), CheckConclusion::Neutral);
    }
}
