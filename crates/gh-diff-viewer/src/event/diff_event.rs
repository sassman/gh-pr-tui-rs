//! Events emitted by the diff viewer for the parent application to handle.

use crate::model::{PendingComment, ReviewEvent};

/// Events emitted by the diff viewer widget.
///
/// The diff viewer is designed to be instrumented - it emits events instead of
/// performing side effects directly. The parent application is responsible for
/// handling these events and performing the necessary actions (e.g., API calls).
///
/// # Example
///
/// ```ignore
/// match diff_viewer_state.handle_key(key) {
///     Some(DiffEvent::RequestContext { file_path, commit_sha, direction, from_line, count }) => {
///         // Fetch additional context lines from GitHub API
///         let lines = github_client.fetch_lines(&file_path, &commit_sha, from_line, count).await?;
///         diff_viewer_state.insert_expanded_lines(&file_path, direction, from_line, lines);
///     }
///     Some(DiffEvent::CommentAdded(comment)) => {
///         // Store the pending comment
///         pending_comments.push(comment);
///     }
///     Some(DiffEvent::SubmitReview { event, body }) => {
///         // Submit the review to GitHub
///         github_client.submit_review(pr_number, event, body, &pending_comments).await?;
///     }
///     Some(DiffEvent::Close) => {
///         // Close the diff viewer
///         current_view = View::PullRequestList;
///     }
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone)]
pub enum DiffEvent {
    /// User requested to expand context (show hidden lines).
    RequestContext {
        /// File path relative to repository root.
        file_path: String,
        /// Commit SHA to fetch from (base or head).
        commit_sha: String,
        /// Direction to expand (up or down).
        direction: ExpandDirection,
        /// Starting line number for the expansion.
        from_line: u32,
        /// Number of lines to fetch.
        count: u32,
    },

    /// User submitted a comment (stored locally, not sent to API yet).
    CommentAdded(PendingComment),

    /// User edited a pending comment.
    CommentEdited {
        /// Index of the comment in the pending comments list.
        index: usize,
        /// New comment body.
        body: String,
    },

    /// User deleted a pending comment.
    CommentDeleted(usize),

    /// User requested to submit the entire review.
    SubmitReview {
        /// Review event type (approve, request changes, comment).
        event: ReviewEvent,
        /// Optional review summary body.
        body: Option<String>,
    },

    /// Navigation changed (useful for status bar updates).
    SelectionChanged {
        /// Currently selected file path.
        file_path: String,
        /// Currently selected line number (if any).
        line: Option<u32>,
    },

    /// User wants to close the diff viewer.
    Close,

    /// File selection changed in the file tree.
    FileSelected {
        /// Path of the selected file.
        file_path: String,
        /// Index of the file in the files list.
        file_index: usize,
    },
}

/// Direction for context expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandDirection {
    /// Expand context above the current position.
    Up,
    /// Expand context below the current position.
    Down,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CommentPosition, DiffSide};

    #[test]
    fn test_diff_event_variants() {
        // Just ensure all variants can be constructed
        let events = [
            DiffEvent::RequestContext {
                file_path: "src/main.rs".to_string(),
                commit_sha: "abc123".to_string(),
                direction: ExpandDirection::Up,
                from_line: 10,
                count: 5,
            },
            DiffEvent::CommentAdded(PendingComment::new(
                "src/main.rs",
                CommentPosition::single(DiffSide::Right, 42),
                "Test comment",
            )),
            DiffEvent::CommentEdited {
                index: 0,
                body: "Updated comment".to_string(),
            },
            DiffEvent::CommentDeleted(0),
            DiffEvent::SubmitReview {
                event: ReviewEvent::Approve,
                body: Some("LGTM".to_string()),
            },
            DiffEvent::SelectionChanged {
                file_path: "src/lib.rs".to_string(),
                line: Some(15),
            },
            DiffEvent::Close,
            DiffEvent::FileSelected {
                file_path: "src/lib.rs".to_string(),
                file_index: 1,
            },
        ];

        assert_eq!(events.len(), 8);
    }
}
