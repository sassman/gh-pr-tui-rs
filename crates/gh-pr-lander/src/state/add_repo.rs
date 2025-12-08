//! Add Repository Form State

use crate::domain_models::Repository;
use gh_pr_config::DEFAULT_HOST;

/// Form field for the add repository dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddRepoField {
    #[default]
    Url,
    Host,
    Org,
    Repo,
    Branch,
}

impl AddRepoField {
    /// Move to the next field
    pub fn next(self) -> Self {
        match self {
            Self::Url => Self::Host,
            Self::Host => Self::Org,
            Self::Org => Self::Repo,
            Self::Repo => Self::Branch,
            Self::Branch => Self::Url,
        }
    }

    /// Move to the previous field
    pub fn prev(self) -> Self {
        match self {
            Self::Url => Self::Branch,
            Self::Host => Self::Url,
            Self::Org => Self::Host,
            Self::Repo => Self::Org,
            Self::Branch => Self::Repo,
        }
    }
}

/// State for the add repository form
#[derive(Debug, Clone, Default)]
pub struct AddRepoFormState {
    pub url: String,    // GitHub URL (for auto-parsing)
    pub host: String,   // GitHub host (empty = github.com)
    pub org: String,    // Organization/owner name
    pub repo: String,   // Repository name
    pub branch: String, // Branch name (default: "main")
    pub focused_field: AddRepoField,
}

impl AddRepoFormState {
    /// Reset the form to its default state
    pub fn reset(&mut self) {
        self.url.clear();
        self.host.clear();
        self.org.clear();
        self.repo.clear();
        self.branch.clear();
        self.focused_field = AddRepoField::default();
    }

    /// Try to parse the URL and populate host/org/repo fields if valid
    ///
    /// Supports formats:
    /// - `https://github.com/org/repo`
    /// - `https://github.example.com/org/repo`
    /// - `git@github.example.com:org/repo.git`
    pub fn parse_url_and_update(&mut self) {
        if let Some((host, org, repo)) = parse_github_url(&self.url) {
            if let Some(h) = host {
                self.host = h;
            } else {
                self.host.clear();
            }
            self.org = org;
            self.repo = repo;
        }
    }

    /// Check if the form is valid (has org and repo)
    pub fn is_valid(&self) -> bool {
        !self.org.is_empty() && !self.repo.is_empty()
    }

    /// Get the branch, defaulting to "main" if empty
    pub fn effective_branch(&self) -> &str {
        if self.branch.is_empty() {
            "main"
        } else {
            &self.branch
        }
    }

    /// Get the host as Option (None for github.com or empty)
    pub fn effective_host(&self) -> Option<String> {
        if self.host.is_empty() || self.host == DEFAULT_HOST {
            None
        } else {
            Some(self.host.clone())
        }
    }

    /// Create a Repository from this form
    pub fn to_repository(&self) -> Repository {
        Repository::with_host(&self.org, &self.repo, self.effective_branch(), self.effective_host())
    }
}

/// Parse a GitHub URL and extract host/org/repo
///
/// Returns (host, org, repo) where host is None for github.com
///
/// Supports:
/// - `https://github.com/org/repo`
/// - `https://github.example.com/org/repo`
/// - `git@github.com:org/repo.git`
/// - `git@github.example.com:org/repo.git`
fn parse_github_url(url: &str) -> Option<(Option<String>, String, String)> {
    let url = url.trim();

    // Try HTTPS format: https://host/org/repo[.git]
    if let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) {
        // Split host from path
        if let Some((host, path)) = rest.split_once('/') {
            if let Some((org, repo)) = parse_org_repo_path(path) {
                let host = if host == DEFAULT_HOST { None } else { Some(host.to_string()) };
                return Some((host, org, repo));
            }
        }
    }

    // Try SSH format: git@host:org/repo[.git]
    if let Some(rest) = url.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            if let Some((org, repo)) = parse_org_repo_path(path) {
                let host = if host == DEFAULT_HOST { None } else { Some(host.to_string()) };
                return Some((host, org, repo));
            }
        }
    }

    // Try short format: host/org/repo (e.g., github.com/org/repo or ghe.example.com/org/repo)
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() >= 3 && parts[0].contains('.') {
        let host = parts[0];
        if let Some((org, repo)) = parse_org_repo_path(&parts[1..].join("/")) {
            let host = if host == DEFAULT_HOST { None } else { Some(host.to_string()) };
            return Some((host, org, repo));
        }
    }

    None
}

/// Parse "org/repo[.git]" into (org, repo)
fn parse_org_repo_path(path: &str) -> Option<(String, String)> {
    // Remove trailing .git if present
    let path = path.strip_suffix(".git").unwrap_or(path);

    // Split by '/'
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        // Take only the first two parts (org/repo), ignore anything after
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https_url() {
        let result = parse_github_url("https://github.com/cargo-generate/cargo-generate.git");
        assert_eq!(
            result,
            Some((None, "cargo-generate".to_string(), "cargo-generate".to_string()))
        );
    }

    #[test]
    fn test_parse_https_url_without_git() {
        let result = parse_github_url("https://github.com/rust-lang/rust");
        assert_eq!(result, Some((None, "rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_ssh_url() {
        let result = parse_github_url("git@github.com:cargo-generate/cargo-generate.git");
        assert_eq!(
            result,
            Some((None, "cargo-generate".to_string(), "cargo-generate".to_string()))
        );
    }

    #[test]
    fn test_parse_ssh_url_without_git() {
        let result = parse_github_url("git@github.com:rust-lang/rust");
        assert_eq!(result, Some((None, "rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_short_url() {
        let result = parse_github_url("github.com/octocat/Hello-World");
        assert_eq!(
            result,
            Some((None, "octocat".to_string(), "Hello-World".to_string()))
        );
    }

    #[test]
    fn test_parse_enterprise_https_url() {
        let result = parse_github_url("https://github.example.com/org/repo");
        assert_eq!(
            result,
            Some((Some("github.example.com".to_string()), "org".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_enterprise_ssh_url() {
        let result = parse_github_url("git@ghe.mycompany.com:team/project.git");
        assert_eq!(
            result,
            Some((Some("ghe.mycompany.com".to_string()), "team".to_string(), "project".to_string()))
        );
    }

    #[test]
    fn test_parse_enterprise_short_url() {
        let result = parse_github_url("ghe.example.com/org/repo");
        assert_eq!(
            result,
            Some((Some("ghe.example.com".to_string()), "org".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_invalid_url() {
        assert_eq!(parse_github_url("invalid"), None);
        assert_eq!(parse_github_url(""), None);
    }

    #[test]
    fn test_to_repository_github_com() {
        let state = AddRepoFormState {
            org: "rust-lang".to_string(),
            repo: "rust".to_string(),
            branch: "master".to_string(),
            ..Default::default()
        };

        let repo = state.to_repository();
        assert_eq!(repo.org, "rust-lang");
        assert_eq!(repo.repo, "rust");
        assert_eq!(repo.branch, "master");
        assert!(repo.is_github_com());
    }

    #[test]
    fn test_to_repository_enterprise() {
        let state = AddRepoFormState {
            host: "ghe.example.com".to_string(),
            org: "team".to_string(),
            repo: "project".to_string(),
            ..Default::default()
        };

        let repo = state.to_repository();
        assert_eq!(repo.org, "team");
        assert_eq!(repo.repo, "project");
        assert_eq!(repo.branch, "main"); // default
        assert!(!repo.is_github_com());
        assert_eq!(repo.effective_host(), "ghe.example.com");
    }
}
