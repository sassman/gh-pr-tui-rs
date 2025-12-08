//! Repository model
//!
//! Domain model for GitHub repositories.

use gh_pr_config::DEFAULT_HOST;
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
    /// GitHub host (None = github.com)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

impl Repository {
    /// Create a new repository (defaults to github.com)
    pub fn new(org: impl Into<String>, repo: impl Into<String>, branch: impl Into<String>) -> Self {
        Self {
            org: org.into(),
            repo: repo.into(),
            branch: branch.into(),
            host: None,
        }
    }

    /// Create a repository with a custom host
    pub fn with_host(
        org: impl Into<String>,
        repo: impl Into<String>,
        branch: impl Into<String>,
        host: Option<String>,
    ) -> Self {
        // Normalize github.com to None
        let host = host.filter(|h| h != DEFAULT_HOST && !h.is_empty());
        Self {
            org: org.into(),
            repo: repo.into(),
            branch: branch.into(),
            host,
        }
    }

    /// Get the effective host (defaults to github.com)
    pub fn effective_host(&self) -> &str {
        self.host.as_deref().unwrap_or(DEFAULT_HOST)
    }

    /// Check if this is a github.com repository
    pub fn is_github_com(&self) -> bool {
        self.host.is_none()
    }

    /// Get the web URL for this repository
    pub fn web_url(&self) -> String {
        format!(
            "https://{}/{}/{}",
            self.effective_host(),
            self.org,
            self.repo
        )
    }

    /// Get the SSH clone URL
    pub fn ssh_url(&self) -> String {
        format!(
            "git@{}:{}/{}.git",
            self.effective_host(),
            self.org,
            self.repo
        )
    }

    /// Get the API base URL for this host
    pub fn api_base_url(&self) -> String {
        if self.is_github_com() {
            "https://api.github.com".to_string()
        } else {
            format!("https://{}/api/v3", self.effective_host())
        }
    }

    /// Display name for the repository (org/repo)
    pub fn display_name(&self) -> String {
        format!("{}/{}", self.org, self.repo)
    }

    /// Full display name with branch and host if not github.com
    pub fn full_display_name(&self) -> String {
        if self.is_github_com() {
            format!("{}/{}@{}", self.org, self.repo, self.branch)
        } else {
            format!(
                "{}:{}/{}@{}",
                self.effective_host(),
                self.org,
                self.repo,
                self.branch
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults_to_github_com() {
        let repo = Repository::new("org", "repo", "main");
        assert!(repo.is_github_com());
        assert_eq!(repo.effective_host(), "github.com");
    }

    #[test]
    fn test_with_host() {
        let repo =
            Repository::with_host("org", "repo", "main", Some("ghe.example.com".to_string()));
        assert!(!repo.is_github_com());
        assert_eq!(repo.effective_host(), "ghe.example.com");
    }

    #[test]
    fn test_web_url() {
        let repo = Repository::new("rust-lang", "rust", "main");
        assert_eq!(repo.web_url(), "https://github.com/rust-lang/rust");

        let repo =
            Repository::with_host("org", "repo", "main", Some("ghe.example.com".to_string()));
        assert_eq!(repo.web_url(), "https://ghe.example.com/org/repo");
    }

    #[test]
    fn test_ssh_url() {
        let repo = Repository::new("rust-lang", "rust", "main");
        assert_eq!(repo.ssh_url(), "git@github.com:rust-lang/rust.git");

        let repo =
            Repository::with_host("org", "repo", "main", Some("ghe.example.com".to_string()));
        assert_eq!(repo.ssh_url(), "git@ghe.example.com:org/repo.git");
    }

    #[test]
    fn test_api_base_url() {
        let repo = Repository::new("org", "repo", "main");
        assert_eq!(repo.api_base_url(), "https://api.github.com");

        let repo =
            Repository::with_host("org", "repo", "main", Some("ghe.example.com".to_string()));
        assert_eq!(repo.api_base_url(), "https://ghe.example.com/api/v3");
    }

    #[test]
    fn test_full_display_name() {
        let repo = Repository::new("org", "repo", "main");
        assert_eq!(repo.full_display_name(), "org/repo@main");

        let repo =
            Repository::with_host("org", "repo", "main", Some("ghe.example.com".to_string()));
        assert_eq!(repo.full_display_name(), "ghe.example.com:org/repo@main");
    }

    #[test]
    fn test_host_normalization() {
        // github.com should be normalized to None
        let repo = Repository::with_host("org", "repo", "main", Some("github.com".to_string()));
        assert!(repo.is_github_com());
        assert!(repo.host.is_none());
    }
}
