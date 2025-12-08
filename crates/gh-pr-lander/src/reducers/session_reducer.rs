//! Session Reducer
//!
//! Handles session-related state changes including loading and restoring
//! previously selected repository and PR.

use crate::actions::SessionAction;
use crate::state::MainViewState;

/// Reduce session actions
pub fn reduce_session(mut state: MainViewState, action: &SessionAction) -> MainViewState {
    match action {
        SessionAction::Loaded {
            selected_repo,
            selected_pr_no,
        } => {
            // Store session selection to restore after repositories load
            state.pending_session_repo = selected_repo.clone();
            state.pending_session_pr_no = *selected_pr_no;
            log::info!(
                "Session loaded: repo={:?}, pr_no={:?}",
                selected_repo,
                selected_pr_no
            );
        }

        SessionAction::RestoreSelection => {
            // Apply pending session selection if repositories match
            if let Some((org, name, branch)) = &state.pending_session_repo {
                for (idx, repo) in state.repositories.iter().enumerate() {
                    if repo.org == *org && repo.repo == *name && repo.branch == *branch {
                        log::info!(
                            "Session: Restoring repository selection to index {} ({}/{})",
                            idx,
                            org,
                            name
                        );
                        state.selected_repository = idx;

                        // Restore PR selection by PR number (not index)
                        if let Some(pr_no) = state.pending_session_pr_no {
                            if let Some(repo_data) = state.repo_data.get_mut(&idx) {
                                // Find the PR by number and get its index
                                if let Some(pr_idx) = repo_data
                                    .prs
                                    .iter()
                                    .position(|pr| pr.number == pr_no)
                                {
                                    repo_data.selected_pr = pr_idx;
                                    log::info!(
                                        "Session: Restoring PR #{} at index {}",
                                        pr_no,
                                        pr_idx
                                    );
                                } else {
                                    log::debug!(
                                        "Session: PR #{} not found in repository, skipping PR restore",
                                        pr_no
                                    );
                                }
                            }
                        }
                        break;
                    }
                }
            }
            // Clear pending session data
            state.pending_session_repo = None;
            state.pending_session_pr_no = None;
        }
    }
    state
}
