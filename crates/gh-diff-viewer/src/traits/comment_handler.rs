//! Trait for handling review comment operations.

use crate::model::{PendingComment, ReviewEvent};
use async_trait::async_trait;
use thiserror::Error;

/// Unique identifier for a submitted comment.
pub type CommentId = String;

/// Errors that can occur during comment operations.
#[derive(Debug, Error)]
pub enum CommentError {
    /// Failed to submit the comment.
    #[error("Failed to submit comment: {0}")]
    SubmissionFailed(String),

    /// The comment was not found.
    #[error("Comment not found: {0}")]
    NotFound(CommentId),

    /// Not authorized to perform the operation.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// The comment handler is not available.
    #[error("Comment handler unavailable: {0}")]
    Unavailable(String),

    /// Rate limited by the API.
    #[error("Rate limited, retry after {0} seconds")]
    RateLimited(u64),
}

/// Handles comment operations (create, edit, delete).
///
/// Implement this trait to allow the diff viewer to submit review comments
/// to the underlying platform (GitHub, GitLab, etc.).
///
/// # Example
///
/// ```ignore
/// struct GithubCommentHandler {
///     client: GithubClient,
///     owner: String,
///     repo: String,
///     pr_number: u64,
///     commit_sha: String,
/// }
///
/// #[async_trait]
/// impl CommentHandler for GithubCommentHandler {
///     async fn submit_comment(&self, comment: PendingComment) -> Result<CommentId, CommentError> {
///         let (start, end) = comment.position.line_range();
///
///         let result = self.client.create_review_comment(
///             &self.owner,
///             &self.repo,
///             self.pr_number,
///             &comment.body,
///             &self.commit_sha,
///             &comment.path,
///             end,
///             comment.position.side.as_github_str(),
///             if start != end { Some(start) } else { None },
///         ).await.map_err(|e| CommentError::SubmissionFailed(e.to_string()))?;
///
///         Ok(result.id.to_string())
///     }
///
///     // ... other methods
/// }
/// ```
#[async_trait]
pub trait CommentHandler: Send + Sync {
    /// Submit a new review comment.
    ///
    /// # Arguments
    /// * `comment` - The pending comment to submit
    ///
    /// # Returns
    /// The ID of the created comment on success.
    async fn submit_comment(&self, comment: PendingComment) -> Result<CommentId, CommentError>;

    /// Edit an existing comment.
    ///
    /// # Arguments
    /// * `id` - The comment ID to edit
    /// * `body` - The new comment body
    async fn edit_comment(&self, id: CommentId, body: String) -> Result<(), CommentError>;

    /// Delete a comment.
    ///
    /// # Arguments
    /// * `id` - The comment ID to delete
    async fn delete_comment(&self, id: CommentId) -> Result<(), CommentError>;

    /// Submit the entire review (with all pending comments).
    ///
    /// # Arguments
    /// * `event` - The review event type (approve, request changes, comment)
    /// * `body` - Optional review summary body
    /// * `comments` - All pending comments to include in the review
    async fn submit_review(
        &self,
        event: ReviewEvent,
        body: Option<String>,
        comments: Vec<PendingComment>,
    ) -> Result<(), CommentError>;

    /// Check if the handler is available.
    fn is_available(&self) -> bool;
}

/// A no-op comment handler for when commenting is disabled or in read-only mode.
#[allow(dead_code)]
pub struct NoOpCommentHandler;

#[async_trait]
impl CommentHandler for NoOpCommentHandler {
    async fn submit_comment(&self, _comment: PendingComment) -> Result<CommentId, CommentError> {
        Err(CommentError::Unavailable(
            "Comment submission is disabled".to_string(),
        ))
    }

    async fn edit_comment(&self, _id: CommentId, _body: String) -> Result<(), CommentError> {
        Err(CommentError::Unavailable(
            "Comment editing is disabled".to_string(),
        ))
    }

    async fn delete_comment(&self, _id: CommentId) -> Result<(), CommentError> {
        Err(CommentError::Unavailable(
            "Comment deletion is disabled".to_string(),
        ))
    }

    async fn submit_review(
        &self,
        _event: ReviewEvent,
        _body: Option<String>,
        _comments: Vec<PendingComment>,
    ) -> Result<(), CommentError> {
        Err(CommentError::Unavailable(
            "Review submission is disabled".to_string(),
        ))
    }

    fn is_available(&self) -> bool {
        false
    }
}
