//! Data models for diff representation.

mod comment;
mod diff;
mod file_tree;

pub use comment::{CommentPosition, DiffSide, PendingComment, ReviewEvent};
pub use diff::{
    DiffLine, DisplayLineInfo, FileDiff, FileStatus, HighlightedSpan, Hunk, LineKind,
    PullRequestDiff,
};
pub use file_tree::{FileTreeNode, FlatFileEntry};
