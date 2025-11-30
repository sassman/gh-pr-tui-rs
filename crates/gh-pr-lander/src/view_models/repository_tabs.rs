//! Repository Tabs View Model
//!
//! Pre-computes all presentation data for the repository tab bar,
//! separating business logic from view rendering.

use crate::domain_models::LoadingState;
use crate::state::AppState;
use ratatui::style::{Color, Modifier, Style};

/// Hourglass icon for loading state
const HOURGLASS_ICON: &str = "⏳";

/// View model for the entire repository tab bar
#[derive(Debug, Clone)]
pub struct RepositoryTabsViewModel {
    /// Pre-computed tabs ready to display
    pub tabs: Vec<TabViewModel>,
    /// Index of the selected tab
    #[allow(dead_code)]
    pub selected_index: usize,
    /// Hint text shown at the end (e.g., "p → a" for add repo)
    pub hint: TabHintViewModel,
}

/// View model for a single tab
#[derive(Debug, Clone)]
pub struct TabViewModel {
    /// Display text (includes loading icon if applicable)
    pub display_text: String,
    /// Style to apply to this tab
    pub style: Style,
    /// Width of the tab in characters (for layout)
    pub width: u16,
}

/// View model for the hint tab at the end
#[derive(Debug, Clone)]
pub struct TabHintViewModel {
    /// Hint text to display
    pub text: String,
    /// Style for the hint
    pub style: Style,
    /// Width of the hint in characters
    pub width: u16,
}

impl RepositoryTabsViewModel {
    /// Build the view model from application state
    pub fn from_state(state: &AppState) -> Self {
        let theme = &state.theme;
        let selected_index = state.main_view.selected_repository;

        // Pre-compute styles
        let active_style = Style::default()
            .fg(theme.bg_primary)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD);

        let inactive_style = Style::default().fg(theme.text_muted).bg(theme.bg_tertiary);

        // Build tabs from repositories
        let tabs: Vec<TabViewModel> = state
            .main_view
            .repositories
            .iter()
            .enumerate()
            .map(|(idx, repo)| {
                let is_selected = idx == selected_index;
                let is_loading = Self::is_repo_loading(state, idx);

                // Build title: "org/repo@branch"
                let title = repo.repo.to_string();

                // Add loading icon if needed
                let display_text = if is_loading {
                    format!("{} {}", HOURGLASS_ICON, title)
                } else {
                    title
                };

                // Calculate width: text + 4 chars padding (2 on each side)
                let width = display_text.chars().count() as u16 + 4;

                let style = if is_selected {
                    active_style
                } else {
                    inactive_style
                };

                TabViewModel {
                    display_text,
                    style,
                    width,
                }
            })
            .collect();

        // Build hint
        let hint = TabHintViewModel {
            text: " p → a ".to_string(),
            style: Style::default()
                .fg(theme.text_muted)
                .add_modifier(Modifier::DIM),
            width: 7, // " p → a " is 7 chars
        };

        Self {
            tabs,
            selected_index,
            hint,
        }
    }

    /// Check if a repository is in a loading state
    fn is_repo_loading(state: &AppState, repo_idx: usize) -> bool {
        state.main_view.repo_data.get(&repo_idx).is_none_or(|data| {
            matches!(
                data.loading_state,
                LoadingState::Idle | LoadingState::Loading
            )
        })
    }

    /// Get the style for active (selected) tabs
    #[allow(dead_code)]
    pub fn active_tab_style(state: &AppState) -> Style {
        let theme = &state.theme;
        Style::default()
            .fg(theme.bg_primary)
            .bg(theme.accent_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Get the style for inactive tabs
    #[allow(dead_code)]
    pub fn inactive_tab_style(state: &AppState) -> Style {
        let theme = &state.theme;
        Style::default().fg(theme.text_muted).bg(theme.bg_tertiary)
    }
}

/// View model for the main view content area
#[derive(Debug, Clone)]
pub enum MainContentViewModel {
    /// Show empty state with a message
    Empty(EmptyStateViewModel),
    /// Show the PR table
    PrTable,
}

/// View model for empty/loading states
#[derive(Debug, Clone)]
pub struct EmptyStateViewModel {
    /// Message to display
    pub message: String,
    /// Border color
    pub border_color: Color,
    /// Text style
    pub text_style: Style,
}

impl EmptyStateViewModel {
    /// Create view model for "no repositories" state
    pub fn no_repos(state: &AppState) -> Self {
        let theme = &state.theme;
        Self {
            message: "No repositories configured. Press 'p → a' to add one.".to_string(),
            border_color: theme.accent_primary,
            text_style: theme.muted(),
        }
    }

    /// Create view model for loading state
    pub fn loading(state: &AppState) -> Self {
        let theme = &state.theme;
        Self {
            message: "Loading pull requests...".to_string(),
            border_color: theme.accent_primary,
            text_style: theme.muted(),
        }
    }

    /// Create view model for "no PRs" state
    pub fn no_prs(state: &AppState) -> Self {
        let theme = &state.theme;
        Self {
            message: "No open pull requests found.".to_string(),
            border_color: theme.accent_primary,
            text_style: theme.muted(),
        }
    }

    /// Create view model for error state
    pub fn error(state: &AppState, error_msg: &str) -> Self {
        let theme = &state.theme;
        Self {
            message: format!("Error: {}. Press Ctrl+r to retry.", error_msg),
            border_color: theme.accent_primary,
            text_style: theme.muted(),
        }
    }
}

/// Determine what content to show in the main view
pub fn determine_main_content(state: &AppState) -> MainContentViewModel {
    let repo_idx = state.main_view.selected_repository;

    // No repositories?
    if state.main_view.repositories.is_empty() {
        return MainContentViewModel::Empty(EmptyStateViewModel::no_repos(state));
    }

    // Get repository data
    let repo_data = state.main_view.repo_data.get(&repo_idx);

    // Check loading state
    match repo_data.map(|rd| &rd.loading_state) {
        None | Some(LoadingState::Idle) | Some(LoadingState::Loading) => {
            MainContentViewModel::Empty(EmptyStateViewModel::loading(state))
        }
        Some(LoadingState::Error(err)) => {
            MainContentViewModel::Empty(EmptyStateViewModel::error(state, err))
        }
        Some(LoadingState::Loaded) => {
            // Check if there are any PRs
            if repo_data.is_some_and(|rd| rd.prs.is_empty()) {
                MainContentViewModel::Empty(EmptyStateViewModel::no_prs(state))
            } else {
                MainContentViewModel::PrTable
            }
        }
    }
}
