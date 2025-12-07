//! GitHub API helper functions
//!
//! This module contains helper functions for GitHub API operations that are used
//! by the middleware layer.

use crate::log::WorkflowRunStatus;
use crate::state::Repo;
use octocrab::Octocrab;

/// Convert check run conclusion string to WorkflowRunStatus
///
/// Helper for parsing check run conclusions from GitHub API
fn parse_check_conclusion(conclusion: &str) -> WorkflowRunStatus {
    match conclusion {
        "success" => WorkflowRunStatus::Success,
        "failure" => WorkflowRunStatus::Failure,
        "cancelled" => WorkflowRunStatus::Cancelled,
        "timed_out" => WorkflowRunStatus::TimedOut,
        "action_required" => WorkflowRunStatus::ActionRequired,
        "skipped" => WorkflowRunStatus::Skipped,
        "neutral" => WorkflowRunStatus::Neutral,
        "stale" => WorkflowRunStatus::Stale,
        "startup_failure" => WorkflowRunStatus::StartupFailure,
        _ => WorkflowRunStatus::Unknown,
    }
}

/// Enable auto-merge on GitHub using GraphQL API
pub async fn enable_github_auto_merge(
    octocrab: &Octocrab,
    repo: &Repo,
    pr_number: usize,
) -> anyhow::Result<()> {
    // First, get the PR's node_id (needed for GraphQL)
    let pr = octocrab
        .pulls(&repo.org, &repo.repo)
        .get(pr_number as u64)
        .await?;

    let node_id = pr
        .node_id
        .ok_or_else(|| anyhow::anyhow!("PR does not have a node_id"))?;

    // GraphQL mutation to enable auto-merge
    let query = format!(
        r#"mutation {{
            enablePullRequestAutoMerge(input: {{
                pullRequestId: "{}",
                mergeMethod: SQUASH
            }}) {{
                pullRequest {{
                    autoMergeRequest {{
                        enabledAt
                    }}
                }}
            }}
        }}"#,
        node_id
    );

    // Execute GraphQL query
    let response: serde_json::Value = octocrab.graphql(&query).await?;

    // Check for errors in response
    if let Some(errors) = response.get("errors") {
        return Err(anyhow::anyhow!("GraphQL error: {}", errors));
    }

    Ok(())
}

/// Get PR CI status by checking commit status
pub async fn get_pr_ci_status(
    octocrab: &Octocrab,
    repo: &Repo,
    head_sha: &str,
) -> anyhow::Result<(String, String)> {
    // Check commit status via check-runs API
    let check_runs_url = format!(
        "/repos/{}/{}/commits/{}/check-runs",
        repo.org, repo.repo, head_sha
    );

    let response: serde_json::Value = octocrab.get(&check_runs_url, None::<&()>).await?;

    let empty_vec = vec![];
    let check_runs = response["check_runs"].as_array().unwrap_or(&empty_vec);

    // Determine overall status
    let mut has_failure = false;
    let mut has_pending = false;
    let mut has_success = false;

    for check in check_runs {
        if let Some(conclusion_str) = check["conclusion"].as_str() {
            let conclusion = parse_check_conclusion(conclusion_str);
            match conclusion {
                WorkflowRunStatus::Success
                | WorkflowRunStatus::Neutral
                | WorkflowRunStatus::Skipped => has_success = true,
                WorkflowRunStatus::Failure
                | WorkflowRunStatus::Cancelled
                | WorkflowRunStatus::TimedOut
                | WorkflowRunStatus::ActionRequired => has_failure = true,
                WorkflowRunStatus::InProgress | WorkflowRunStatus::Unknown => has_pending = true,
                WorkflowRunStatus::Stale | WorkflowRunStatus::StartupFailure => has_failure = true,
            }
        } else if let Some(status) = check["status"].as_str()
            && (status == "in_progress" || status == "queued")
        {
            has_pending = true;
        }
    }

    let overall_status = if has_failure {
        ("completed".to_string(), "failure".to_string())
    } else if has_pending {
        ("in_progress".to_string(), "pending".to_string())
    } else if has_success {
        ("completed".to_string(), "success".to_string())
    } else {
        ("unknown".to_string(), "unknown".to_string())
    };

    Ok(overall_status)
}
