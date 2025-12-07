//! View model for PR table view
//!
//! Separates presentation logic from domain models and view rendering.
//! Pre-computes all display text, colors, and styles in the view model.

use crate::domain_models::{LoadingState, MergeableStatus, Pr, Repository};
use crate::state::RepositoryData;
use gh_pr_lander_theme::Theme;
use ratatui::style::Color;

/// View model for the entire PR table
#[derive(Debug, Clone)]
pub struct PrTableViewModel {
    /// Header with title and status
    pub header: PrTableHeaderViewModel,
    /// Pre-computed rows ready to display
    pub rows: Vec<PrRowViewModel>,
    /// Current cursor position (for keyboard navigation)
    pub selected_index: usize,
}

/// View model for table header
#[derive(Debug, Clone)]
pub struct PrTableHeaderViewModel {
    /// Title text: "GitHub PRs: org/repo@branch"
    pub title: String,
    /// Status text: "Loaded [Ctrl+r to refresh]", etc.
    pub status_text: String,
    /// Status color (from theme)
    pub status_color: Color,
}

/// View model for a single PR row
#[derive(Debug, Clone)]
pub struct PrRowViewModel {
    /// Pre-formatted cell texts
    pub pr_number: String, // "#123"
    pub title: String,       // "Fix: broken tests"
    pub author: String,      // "sassman"
    pub comments: String,    // "5"
    pub status_text: String, // "✓ Ready"

    /// Pre-computed styles
    pub bg_color: Color, // Background (alternating, selected, etc.)
    pub fg_color: Color,     // Text color
    pub status_color: Color, // Status-specific color
    pub additions: usize,    // Raw additions count (for coloring)
    pub deletions: usize,    // Raw deletions count (for coloring)
}

impl PrTableViewModel {
    /// Transform state into display-ready view model
    pub fn from_repo_data(repo_data: &RepositoryData, repo: &Repository, theme: &Theme) -> Self {
        // Build header
        let header = Self::build_header(repo_data, repo, theme);

        // Build rows
        let rows = repo_data
            .prs
            .iter()
            .enumerate()
            .map(|(index, pr)| {
                let is_multi_selected = repo_data.selected_pr_numbers.contains(&pr.number);
                Self::build_row(pr, index, repo_data.selected_pr, is_multi_selected, theme)
            })
            .collect();

        Self {
            header,
            rows,
            selected_index: repo_data.selected_pr,
        }
    }

    fn build_header(
        repo_data: &RepositoryData,
        repo: &Repository,
        theme: &Theme,
    ) -> PrTableHeaderViewModel {
        let title = format!("  {}/{}@{} ", repo.org, repo.repo, repo.branch);

        let (status_text, status_color) = Self::format_loading_state(
            &repo_data.loading_state,
            repo_data.last_updated.as_ref(),
            theme,
        );

        PrTableHeaderViewModel {
            title,
            status_text,
            status_color,
        }
    }

    fn build_row(
        pr: &Pr,
        index: usize,
        cursor_index: usize,
        is_multi_selected: bool,
        theme: &Theme,
    ) -> PrRowViewModel {
        let is_cursor = index == cursor_index;

        // Pre-compute display text with selection indicator
        let selection_indicator = if is_multi_selected { "●" } else { " " };
        let pr_number = format!("{} #{}", selection_indicator, pr.number);
        let title = pr.title.clone();
        let author = pr.author.clone();
        let comments = pr.comments.to_string();

        // Format status with icon and label
        let status_text = format!("{} {}", pr.mergeable.icon(), pr.mergeable.label());
        let status_color = Self::mergeable_status_color(pr.mergeable, theme);

        // Compute colors - multi-selected rows get highlighted differently
        let (fg_color, bg_color) = if is_cursor {
            (theme.active_fg, theme.selected_bg)
        } else if is_multi_selected {
            // Multi-selected but not cursor: subtle highlight
            (
                theme.text().fg.unwrap_or(Color::White),
                Color::Rgb(40, 50, 60),
            )
        } else {
            // Alternating row colors
            let bg = if index.is_multiple_of(2) {
                Color::Reset
            } else {
                Color::Rgb(30, 30, 40) // Subtle alternate row color
            };
            (theme.text().fg.unwrap_or(Color::White), bg)
        };

        PrRowViewModel {
            pr_number,
            title,
            author,
            comments,
            status_text,
            bg_color,
            fg_color,
            status_color,
            additions: pr.additions,
            deletions: pr.deletions,
        }
    }

    /// Format loading state for display
    fn format_loading_state(
        state: &LoadingState,
        last_updated: Option<&chrono::DateTime<chrono::Local>>,
        theme: &Theme,
    ) -> (String, Color) {
        match state {
            LoadingState::Idle => (
                "Idle [Ctrl+r to refresh]".to_string(),
                theme.muted().fg.unwrap_or(Color::Gray),
            ),
            LoadingState::Loading => ("Loading...".to_string(), Color::Yellow),
            LoadingState::Loaded => {
                let status_text = if let Some(timestamp) = last_updated {
                    format!(
                        "Updated {} [Ctrl+r to refresh]",
                        timestamp.format("%H:%M:%S")
                    )
                } else {
                    "Loaded [Ctrl+r to refresh]".to_string()
                };
                (status_text, Color::Green)
            }
            LoadingState::Error(err) => {
                let err_short = if err.len() > 30 {
                    format!("{}...", &err[..30])
                } else {
                    err.clone()
                };
                (
                    format!("Error: {} [Ctrl+r to retry]", err_short),
                    Color::Red,
                )
            }
        }
    }

    /// Get color for mergeable status
    fn mergeable_status_color(status: MergeableStatus, theme: &Theme) -> Color {
        match status {
            MergeableStatus::Unknown => theme.muted().fg.unwrap_or(Color::Gray),
            MergeableStatus::Checking => Color::Yellow,
            MergeableStatus::Ready => Color::Green,
            MergeableStatus::NeedsRebase => Color::Yellow,
            MergeableStatus::BuildFailed => Color::Red,
            MergeableStatus::Conflicted => Color::Red,
            MergeableStatus::Blocked => Color::Red,
            MergeableStatus::Rebasing => Color::Cyan,
            MergeableStatus::Merging => Color::Cyan,
        }
    }
}
