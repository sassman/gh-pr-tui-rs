//! Repository model
//!
//! Domain model for GitHub repositories.

use serde::{Deserialize, Serialize};

/// A tracked GitHub repository
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Repository {
    /// Organization or owner name
    pub org: String,
    /// Repository name
    pub repo: String,
    /// Branch name (default: "main")
    pub branch: String,
}

impl Repository {
    /// Create a new repository
    pub fn new(org: impl Into<String>, repo: impl Into<String>, branch: impl Into<String>) -> Self {
        Self {
            org: org.into(),
            repo: repo.into(),
            branch: branch.into(),
        }
    }

    /// Display name for the repository (org/repo)
    pub fn display_name(&self) -> String {
        format!("{}/{}", self.org, self.repo)
    }

    /// Full display name with branch (org/repo@branch)
    pub fn full_display_name(&self) -> String {
        format!("{}/{}@{}", self.org, self.repo, self.branch)
    }
}
