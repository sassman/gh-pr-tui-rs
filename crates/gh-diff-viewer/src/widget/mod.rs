//! Ratatui widgets for the diff viewer.

mod diff_content;
mod diff_viewer;
mod file_tree;
mod review_popup;

pub use diff_content::{DiffContentWidget, DiffRenderData, FooterHint};
pub use diff_viewer::DiffViewer;
pub use file_tree::FileTreeWidget;
pub use review_popup::ReviewPopupWidget;
