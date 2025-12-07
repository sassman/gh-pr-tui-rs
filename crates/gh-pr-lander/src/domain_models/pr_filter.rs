//! PR Filter model
//!
//! Filtering options for pull requests.

use serde::{Deserialize, Serialize};

/// Filter for categorizing and displaying PRs
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum PrFilter {
    /// Show all PRs
    #[default]
    None,
    /// Show only feature PRs
    Feat,
    /// Show only fix PRs
    Fix,
    /// Show only chore PRs
    Chore,
}

#[allow(dead_code)]
impl PrFilter {
    /// Check if a PR title matches this filter
    pub fn matches(&self, title: &str) -> bool {
        match self {
            PrFilter::None => true,
            PrFilter::Feat => title.to_lowercase().contains("feat"),
            PrFilter::Fix => title.to_lowercase().contains("fix"),
            PrFilter::Chore => title.to_lowercase().contains("chore"),
        }
    }

    /// Get the display label for this filter
    pub fn label(&self) -> &str {
        match self {
            PrFilter::None => "All",
            PrFilter::Feat => "Feat",
            PrFilter::Fix => "Fix",
            PrFilter::Chore => "Chore",
        }
    }

    /// Cycle to the next filter option
    pub fn next(&self) -> Self {
        match self {
            PrFilter::None => PrFilter::Feat,
            PrFilter::Feat => PrFilter::Fix,
            PrFilter::Fix => PrFilter::Chore,
            PrFilter::Chore => PrFilter::None,
        }
    }

    /// Cycle to the previous filter option
    pub fn prev(&self) -> Self {
        match self {
            PrFilter::None => PrFilter::Chore,
            PrFilter::Feat => PrFilter::None,
            PrFilter::Fix => PrFilter::Feat,
            PrFilter::Chore => PrFilter::Fix,
        }
    }
}
