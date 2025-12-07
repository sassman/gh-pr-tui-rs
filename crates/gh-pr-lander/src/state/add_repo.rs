//! Add Repository Form State

use crate::domain_models::Repository;

/// Form field for the add repository dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddRepoField {
    #[default]
    Url,
    Org,
    Repo,
    Branch,
}

impl AddRepoField {
    /// Move to the next field
    pub fn next(self) -> Self {
        match self {
            Self::Url => Self::Org,
            Self::Org => Self::Repo,
            Self::Repo => Self::Branch,
            Self::Branch => Self::Url,
        }
    }

    /// Move to the previous field
    pub fn prev(self) -> Self {
        match self {
            Self::Url => Self::Branch,
            Self::Org => Self::Url,
            Self::Repo => Self::Org,
            Self::Branch => Self::Repo,
        }
    }
}

/// State for the add repository form
#[derive(Debug, Clone, Default)]
pub struct AddRepoFormState {
    pub url: String,    // GitHub URL (for auto-parsing)
    pub org: String,    // Organization/owner name
    pub repo: String,   // Repository name
    pub branch: String, // Branch name (default: "main")
    pub focused_field: AddRepoField,
}

impl AddRepoFormState {
    /// Reset the form to its default state
    pub fn reset(&mut self) {
        self.url.clear();
        self.org.clear();
        self.repo.clear();
        self.branch.clear();
        self.focused_field = AddRepoField::default();
    }

    /// Try to parse the URL and populate org/repo fields if valid
    ///
    /// Supports formats:
    /// - `https://github.com/org/repo`
    /// - `https://github.com/org/repo.git`
    /// - `git@github.com:org/repo.git`
    /// - `git@github.com:org/repo`
    pub fn parse_url_and_update(&mut self) {
        if let Some((org, repo)) = parse_github_url(&self.url) {
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

    /// Create a Repository from this form
    pub fn to_repository(&self) -> Repository {
        Repository::new(&self.org, &self.repo, self.effective_branch())
    }
}

/// Parse a GitHub URL and extract org/repo
///
/// Supports:
/// - `https://github.com/org/repo`
/// - `https://github.com/org/repo.git`
/// - `git@github.com:org/repo.git`
/// - `git@github.com:org/repo`
fn parse_github_url(url: &str) -> Option<(String, String)> {
    let url = url.trim();

    // Try HTTPS format: https://github.com/org/repo[.git]
    if let Some(rest) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        return parse_org_repo_path(rest);
    }

    // Try SSH format: git@github.com:org/repo[.git]
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        return parse_org_repo_path(rest);
    }

    // Try short format: github.com/org/repo
    if let Some(rest) = url.strip_prefix("github.com/") {
        return parse_org_repo_path(rest);
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
            Some(("cargo-generate".to_string(), "cargo-generate".to_string()))
        );
    }

    #[test]
    fn test_parse_https_url_without_git() {
        let result = parse_github_url("https://github.com/rust-lang/rust");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_ssh_url() {
        let result = parse_github_url("git@github.com:cargo-generate/cargo-generate.git");
        assert_eq!(
            result,
            Some(("cargo-generate".to_string(), "cargo-generate".to_string()))
        );
    }

    #[test]
    fn test_parse_ssh_url_without_git() {
        let result = parse_github_url("git@github.com:rust-lang/rust");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_short_url() {
        let result = parse_github_url("github.com/octocat/Hello-World");
        assert_eq!(
            result,
            Some(("octocat".to_string(), "Hello-World".to_string()))
        );
    }

    #[test]
    fn test_parse_invalid_url() {
        assert_eq!(parse_github_url("invalid"), None);
        assert_eq!(parse_github_url("https://gitlab.com/org/repo"), None);
        assert_eq!(parse_github_url(""), None);
    }
}
