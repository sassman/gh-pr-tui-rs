use ratatui::style::{palette::tailwind, Color};

/// View model for splash screen - all presentation data pre-computed
#[derive(Debug, Clone)]
pub struct SplashScreenViewModel {
    /// Title text (always "PR Bulk Review TUI")
    pub title: String,
    /// Stage-specific message
    pub stage_message: String,
    /// Progress percentage (0-100)
    pub progress_percent: usize,
    /// Whether this is an error state
    pub is_error: bool,
    /// Spinner character or error icon
    pub spinner_text: String,
    /// Pre-formatted progress bar (e.g., "▰▰▰▰▱▱▱  50%")
    pub progress_bar: String,
    /// Pre-computed colors
    pub title_color: Color,
    pub spinner_color: Color,
    pub message_color: Color,
    pub progress_bar_color: Color,
}

impl SplashScreenViewModel {
    const SPINNER_FRAMES: [&'static str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    /// Build view model from bootstrap state
    pub fn from_state(
        bootstrap_state: &crate::state::BootstrapState,
        repos: &[crate::state::Repo],
        selected_repo: usize,
        spinner_frame: usize,
        bar_width: usize,
        theme: &crate::theme::Theme,
    ) -> Self {
        // Determine stage info from bootstrap state
        let (stage_message, progress, is_error) = match bootstrap_state {
            crate::state::BootstrapState::NotStarted => {
                ("Initializing application...".to_string(), 0, false)
            }
            crate::state::BootstrapState::LoadingRepositories => {
                ("Loading repositories...".to_string(), 25, false)
            }
            crate::state::BootstrapState::RestoringSession => {
                ("Restoring session...".to_string(), 50, false)
            }
            crate::state::BootstrapState::LoadingFirstRepo => {
                // Loading the selected repo first
                if let Some(repo) = repos.get(selected_repo) {
                    (format!("Loading {}...", repo.repo), 75, false)
                } else {
                    ("Loading repository...".to_string(), 75, false)
                }
            }
            crate::state::BootstrapState::UIReady
            | crate::state::BootstrapState::LoadingRemainingRepos
            | crate::state::BootstrapState::Completed => {
                // This state shouldn't be shown in splash screen as UI should be visible
                ("Ready!".to_string(), 100, false)
            }
            crate::state::BootstrapState::Error(err) => {
                (format!("Error: {}", err), 0, true)
            }
        };

        // Spinner or error icon
        let spinner_text = if is_error {
            "✗ Error".to_string()
        } else {
            let spinner_char = Self::SPINNER_FRAMES[spinner_frame % Self::SPINNER_FRAMES.len()];
            format!("{} Loading...", spinner_char)
        };

        // Pre-format progress bar
        let progress_bar = if is_error {
            String::new()
        } else {
            let filled = (bar_width * progress) / 100;
            let empty = bar_width.saturating_sub(filled);
            format!("{}{}  {}%", "▰".repeat(filled), "▱".repeat(empty), progress)
        };

        // Pre-compute colors
        let title_color = tailwind::BLUE.c400;
        let spinner_color = if is_error {
            theme.status_error
        } else {
            theme.status_warning
        };
        let message_color = if is_error {
            theme.status_error
        } else {
            theme.text_secondary
        };
        let progress_bar_color = theme.status_info;

        Self {
            title: "PR Bulk Review TUI".to_string(),
            stage_message,
            progress_percent: progress,
            is_error,
            spinner_text,
            progress_bar,
            title_color,
            spinner_color,
            message_color,
            progress_bar_color,
        }
    }
}
