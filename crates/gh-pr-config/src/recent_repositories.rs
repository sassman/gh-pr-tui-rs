//! Recent repositories management
//!
//! Handles loading and saving recently used repositories.

#[allow(deprecated)] // Intentionally using legacy path until migration complete
use crate::files::{create_recent_repositories_file, open_recent_repositories_file};
use crate::DEFAULT_HOST;
use serde::{Deserialize, Serialize};
use std::io::BufReader;

/// A recently used repository entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRepository {
    /// GitHub organization or user name
    pub org: String,
    /// Repository name
    pub repo: String,
    /// Branch name (default: "main")
    #[serde(default = "default_branch")]
    pub branch: String,
    /// GitHub host (None = github.com)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

fn default_branch() -> String {
    "main".to_string()
}

impl RecentRepository {
    pub fn new(org: impl Into<String>, repo: impl Into<String>, branch: impl Into<String>) -> Self {
        Self {
            org: org.into(),
            repo: repo.into(),
            branch: branch.into(),
            host: None,
        }
    }

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
}

/// Load recent repositories from the config file
///
/// Returns an empty vector if the file doesn't exist or can't be parsed.
pub fn load_recent_repositories() -> Vec<RecentRepository> {
    #[allow(deprecated)] // Intentionally using legacy path until migration complete
    match open_recent_repositories_file() {
        Ok(file) => {
            let reader = BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(repos) => {
                    log::info!("Loaded recent repositories from .gh-pr-lander.repos.json");
                    repos
                }
                Err(e) => {
                    log::warn!("Failed to parse recent repositories file: {}", e);
                    Vec::new()
                }
            }
        }
        Err(_) => {
            log::debug!("No recent repositories file found, starting fresh");
            Vec::new()
        }
    }
}

/// Save recent repositories to the config file
///
/// Returns an error if the file cannot be created or written.
pub fn save_recent_repositories(repos: &[RecentRepository]) -> anyhow::Result<()> {
    #[allow(deprecated)] // Intentionally using legacy path until migration complete
    let file = create_recent_repositories_file()?;
    serde_json::to_writer_pretty(file, repos)?;
    log::info!(
        "Saved {} recent repositories to .gh-pr-lander.repos.json",
        repos.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recent_repository_new() {
        let repo = RecentRepository::new("rust-lang", "rust", "master");
        assert_eq!(repo.org, "rust-lang");
        assert_eq!(repo.repo, "rust");
        assert_eq!(repo.branch, "master");
    }

    #[test]
    fn test_recent_repository_serde() {
        let repo = RecentRepository::new("octocat", "Hello-World", "main");
        let json = serde_json::to_string(&repo).unwrap();
        let parsed: RecentRepository = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.org, "octocat");
        assert_eq!(parsed.repo, "Hello-World");
        assert_eq!(parsed.branch, "main");
    }

    #[test]
    fn test_default_branch() {
        let json = r#"{"org": "test", "repo": "repo"}"#;
        let parsed: RecentRepository = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.branch, "main");
    }

    #[test]
    fn test_with_host() {
        let repo = RecentRepository::with_host(
            "org",
            "repo",
            "main",
            Some("github.example.com".to_string()),
        );
        assert_eq!(repo.host, Some("github.example.com".to_string()));
        assert_eq!(repo.effective_host(), "github.example.com");
        assert!(!repo.is_github_com());
    }

    #[test]
    fn test_host_normalization() {
        // github.com should be normalized to None
        let repo =
            RecentRepository::with_host("org", "repo", "main", Some("github.com".to_string()));
        assert!(repo.host.is_none());
        assert!(repo.is_github_com());

        // Empty string should be normalized to None
        let repo = RecentRepository::with_host("org", "repo", "main", Some("".to_string()));
        assert!(repo.host.is_none());
    }

    #[test]
    fn test_host_serde_skip_none() {
        let repo = RecentRepository::new("org", "repo", "main");
        let json = serde_json::to_string(&repo).unwrap();
        // host should not appear in JSON when None
        assert!(!json.contains("host"));
    }

    #[test]
    fn test_host_serde_with_value() {
        let repo =
            RecentRepository::with_host("org", "repo", "main", Some("ghe.example.com".to_string()));
        let json = serde_json::to_string(&repo).unwrap();
        assert!(json.contains("ghe.example.com"));

        let parsed: RecentRepository = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.host, Some("ghe.example.com".to_string()));
    }
}
