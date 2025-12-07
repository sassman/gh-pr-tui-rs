//! Diff Viewer Middleware
//!
//! Handles side effects for the diff viewer, including:
//! - Close logic when Escape is pressed and there's nothing to cancel/escape from
//! - Comment submission when Confirm is pressed while editing a comment
//! - Review submission when Confirm is pressed in the review popup

use crate::actions::{Action, DiffViewerAction, GlobalAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;

/// Middleware for diff viewer side effects
pub struct DiffViewerMiddleware;

impl DiffViewerMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiffViewerMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for DiffViewerMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Handle Escape: close view if nothing to escape from
            Action::DiffViewer(DiffViewerAction::EscapeOrFocusTree) => {
                // Check current state to decide if we should close the view
                // This runs BEFORE the reducer, so we check the pre-action state
                if let Some(ref inner) = state.diff_viewer.inner {
                    if inner.is_editing_comment() {
                        // Let reducer handle: cancel comment
                        return true;
                    }
                    if inner.show_review_popup {
                        // Let reducer handle: hide review popup
                        return true;
                    }
                    if !inner.nav.file_tree_focused {
                        // Let reducer handle: focus file tree
                        return true;
                    }
                    // Already in file tree with nothing to cancel - close the view
                    log::debug!("DiffViewerMiddleware: EscapeOrFocusTree - closing view");
                    dispatcher.dispatch(Action::Global(GlobalAction::Close));
                    false // Consume the action
                } else {
                    // No inner state - close the view
                    log::debug!(
                        "DiffViewerMiddleware: EscapeOrFocusTree with no inner - closing view"
                    );
                    dispatcher.dispatch(Action::Global(GlobalAction::Close));
                    false
                }
            }

            // Handle Confirm: either submit comment or submit review
            Action::DiffViewer(DiffViewerAction::Confirm) => {
                if let Some(ref inner) = state.diff_viewer.inner {
                    // Check if editing a comment - submit or delete based on content
                    if let Some(ref editor) = inner.comment_editor {
                        if let Some(pr_number) = state.diff_viewer.pr_number {
                            if editor.body.trim().is_empty() {
                                // Empty comment body
                                if let Some(github_id) = editor.github_id {
                                    // Editing existing posted comment - delete it
                                    log::debug!(
                                        "DiffViewerMiddleware: Confirm with empty body on existing comment - dispatching DeleteCommentRequest"
                                    );
                                    dispatcher.dispatch(Action::DiffViewer(
                                        DiffViewerAction::DeleteCommentRequest {
                                            pr_number,
                                            github_id,
                                            path: editor.file_path.clone(),
                                            line: editor.position.line,
                                            side: editor.position.side.as_github_str().to_string(),
                                        },
                                    ));
                                }
                                // Empty body without github_id - just close the editor (no API call)
                            } else if let Some(ref head_sha) = state.diff_viewer.head_sha {
                                // Non-empty comment - submit to GitHub
                                log::debug!(
                                    "DiffViewerMiddleware: Confirm with comment - dispatching SubmitCommentRequest"
                                );
                                dispatcher.dispatch(Action::DiffViewer(
                                    DiffViewerAction::SubmitCommentRequest {
                                        pr_number,
                                        head_sha: head_sha.clone(),
                                        path: editor.file_path.clone(),
                                        line: editor.position.line,
                                        side: editor.position.side.as_github_str().to_string(),
                                        body: editor.body.clone(),
                                    },
                                ));
                            }
                        }
                        // Let the Confirm action pass through to close the editor
                        // (reducer will clear the comment_editor)
                        return true;
                    }

                    // Check if review popup is visible - submit review
                    if inner.show_review_popup {
                        if let Some(pr_number) = state.diff_viewer.pr_number {
                            log::debug!(
                                "DiffViewerMiddleware: Confirm in review popup - dispatching SubmitReviewRequest"
                            );
                            dispatcher.dispatch(Action::DiffViewer(
                                DiffViewerAction::SubmitReviewRequest {
                                    pr_number,
                                    event: inner.selected_review_event,
                                },
                            ));
                        }
                        // Let the Confirm action pass through to close the popup
                        return true;
                    }
                }
                // Not in comment editor or review popup - let action pass through
                true
            }

            // All other actions pass through
            _ => true,
        }
    }
}
