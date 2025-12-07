//! Comment-related data structures for PR reviews.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A pending (not yet submitted) review comment.
#[derive(Debug, Clone)]
pub struct PendingComment {
    /// Unique identifier for this pending comment.
    pub id: Uuid,
    /// GitHub comment ID (set after posting to GitHub).
    pub github_id: Option<u64>,
    /// File path.
    pub path: String,
    /// Position information.
    pub position: CommentPosition,
    /// Comment body (markdown).
    pub body: String,
    /// When the comment was created locally.
    pub created_at: DateTime<Utc>,
}

impl PendingComment {
    /// Create a new pending comment.
    pub fn new(
        path: impl Into<String>,
        position: CommentPosition,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            github_id: None,
            path: path.into(),
            position,
            body: body.into(),
            created_at: Utc::now(),
        }
    }

    /// Create from an existing GitHub comment.
    pub fn from_github(
        github_id: u64,
        path: impl Into<String>,
        position: CommentPosition,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            github_id: Some(github_id),
            path: path.into(),
            position,
            body: body.into(),
            created_at: Utc::now(),
        }
    }
}

/// Where the comment is anchored in the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentPosition {
    /// Which side of the diff (for split view / GitHub API).
    pub side: DiffSide,
    /// Line number (in the respective file version).
    pub line: u32,
    /// For multi-line comments: starting line.
    pub start_line: Option<u32>,
}

impl CommentPosition {
    /// Create a single-line comment position.
    pub fn single(side: DiffSide, line: u32) -> Self {
        Self {
            side,
            line,
            start_line: None,
        }
    }

    /// Create a multi-line comment position.
    pub fn range(side: DiffSide, start_line: u32, end_line: u32) -> Self {
        Self {
            side,
            line: end_line,
            start_line: Some(start_line),
        }
    }

    /// Check if this is a multi-line comment.
    pub fn is_multiline(&self) -> bool {
        self.start_line.is_some()
    }

    /// Get the line range as (start, end).
    pub fn line_range(&self) -> (u32, u32) {
        (self.start_line.unwrap_or(self.line), self.line)
    }
}

/// Which side of the diff the comment is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    /// Old file (deletions side).
    Left,
    /// New file (additions side).
    Right,
}

impl DiffSide {
    /// Convert to GitHub API string representation.
    pub fn as_github_str(&self) -> &'static str {
        match self {
            DiffSide::Left => "LEFT",
            DiffSide::Right => "RIGHT",
        }
    }
}

/// The type of review to submit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewEvent {
    /// Approve the pull request.
    Approve,
    /// Request changes.
    RequestChanges,
    /// Just leave comments (neutral).
    Comment,
}

impl ReviewEvent {
    /// Convert to GitHub API string representation.
    pub fn as_github_str(&self) -> &'static str {
        match self {
            ReviewEvent::Approve => "APPROVE",
            ReviewEvent::RequestChanges => "REQUEST_CHANGES",
            ReviewEvent::Comment => "COMMENT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_position_single() {
        let pos = CommentPosition::single(DiffSide::Right, 42);
        assert!(!pos.is_multiline());
        assert_eq!(pos.line_range(), (42, 42));
    }

    #[test]
    fn test_comment_position_multiline() {
        let pos = CommentPosition::range(DiffSide::Right, 10, 20);
        assert!(pos.is_multiline());
        assert_eq!(pos.line_range(), (10, 20));
    }

    #[test]
    fn test_diff_side_github_str() {
        assert_eq!(DiffSide::Left.as_github_str(), "LEFT");
        assert_eq!(DiffSide::Right.as_github_str(), "RIGHT");
    }

    #[test]
    fn test_review_event_github_str() {
        assert_eq!(ReviewEvent::Approve.as_github_str(), "APPROVE");
        assert_eq!(
            ReviewEvent::RequestChanges.as_github_str(),
            "REQUEST_CHANGES"
        );
        assert_eq!(ReviewEvent::Comment.as_github_str(), "COMMENT");
    }
}
