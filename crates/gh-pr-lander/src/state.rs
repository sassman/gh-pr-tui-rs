use crate::domain_models::Repository;
use crate::keybindings::{default_keymap, Keymap};
use crate::logger::OwnedLogRecord;
use crate::views::{SplashView, View};

/// Debug console state
#[derive(Debug, Clone, Default)]
pub struct DebugConsoleState {
    pub visible: bool,
    pub logs: Vec<OwnedLogRecord>,
    pub scroll_offset: usize, // Current scroll position (0 = bottom/latest)
}

/// Splash screen state
#[derive(Debug, Clone)]
pub struct SplashState {
    pub bootstrapping: bool,
    pub animation_frame: usize, // Current frame of the snake animation (0-15)
}

impl Default for SplashState {
    fn default() -> Self {
        Self {
            bootstrapping: true,
            animation_frame: 0,
        }
    }
}

/// Main view state
#[derive(Debug, Clone, Default)]
pub struct MainViewState {
    pub selected_repository: usize, // Currently selected repository index
    pub repositories: Vec<Repository>, // List of tracked repositories
    pub repo_data: std::collections::HashMap<usize, RepositoryData>, // PR data per repository
}

/// Data for a single repository (PRs, loading state, etc.)
#[derive(Debug, Clone, Default)]
pub struct RepositoryData {
    /// List of pull requests for this repository
    pub prs: Vec<crate::domain_models::Pr>,
    /// Current loading state
    pub loading_state: crate::domain_models::LoadingState,
    /// Currently selected PR index in the table (cursor position)
    pub selected_pr: usize,
    /// Set of selected PR numbers for bulk operations
    pub selected_pr_numbers: std::collections::HashSet<usize>,
    /// Timestamp of last successful load
    pub last_updated: Option<chrono::DateTime<chrono::Local>>,
}

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
    /// - https://github.com/org/repo
    /// - https://github.com/org/repo.git
    /// - git@github.com:org/repo.git
    /// - git@github.com:org/repo
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
/// - https://github.com/org/repo
/// - https://github.com/org/repo.git
/// - git@github.com:org/repo.git
/// - git@github.com:org/repo
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

/// Command palette state
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub query: String,         // Search query
    pub selected_index: usize, // Currently selected command index
}

/// Application state
pub struct AppState {
    pub running: bool,
    /// Stack of views - bottom view is the base, top views are floating overlays
    /// Views are rendered bottom-up, so the last view in the stack renders on top
    pub view_stack: Vec<Box<dyn View>>,
    pub splash: SplashState,
    pub main_view: MainViewState,
    pub debug_console: DebugConsoleState,
    pub command_palette: CommandPaletteState,
    pub add_repo_form: AddRepoFormState,
    pub theme: crate::theme::Theme,
    /// The keymap containing all keybindings
    pub keymap: Keymap,
}

impl AppState {
    /// Get the top-most (active) view from the stack
    pub fn active_view(&self) -> &dyn View {
        self.view_stack
            .last()
            .expect("View stack should never be empty")
            .as_ref()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("running", &self.running)
            .field("view_stack", &format!("{} views", self.view_stack.len()))
            .field("splash", &self.splash)
            .field("main_view", &self.main_view)
            .field("debug_console", &self.debug_console)
            .field("command_palette", &self.command_palette)
            .field("add_repo_form", &self.add_repo_form)
            .field("theme", &"<theme>")
            .finish()
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            running: self.running,
            view_stack: self.view_stack.clone(),
            splash: self.splash.clone(),
            main_view: self.main_view.clone(),
            debug_console: self.debug_console.clone(),
            command_palette: self.command_palette.clone(),
            add_repo_form: self.add_repo_form.clone(),
            theme: self.theme.clone(),
            keymap: self.keymap.clone(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            view_stack: vec![Box::new(SplashView::new())],
            splash: SplashState::default(),
            main_view: MainViewState::default(),
            debug_console: DebugConsoleState::default(),
            command_palette: CommandPaletteState::default(),
            add_repo_form: AddRepoFormState::default(),
            theme: crate::theme::Theme::default(),
            keymap: default_keymap(),
        }
    }
}
