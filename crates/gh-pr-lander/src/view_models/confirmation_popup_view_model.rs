//! View model for confirmation popup
//!
//! Pre-computes all display data for the confirmation popup view,
//! separating data preparation from rendering logic.

use crate::state::ConfirmationPopupState;
use ratatui::style::Color;

/// View model for the confirmation popup
#[derive(Debug, Clone)]
pub struct ConfirmationPopupViewModel {
    /// Popup title (e.g., "Approve Pull Request")
    pub title: String,
    /// Target info line (e.g., "Approving: PR #123" or "Approving: 3 PRs")
    pub target_line: String,
    /// Instructions text (e.g., "Enter your approval message:")
    pub instructions: String,
    /// Input label (e.g., "Message:")
    pub input_label: String,
    /// Current input value
    pub input_value: String,
    /// Whether input is empty (for placeholder styling)
    #[allow(dead_code)]
    pub input_is_empty: bool,
    /// Whether the form is valid for submission
    #[allow(dead_code)]
    pub is_valid: bool,
    /// Validation hint if not valid
    pub validation_hint: Option<String>,
    /// Footer hints for keyboard shortcuts
    pub footer_hints: ConfirmationFooterHints,
    /// Theme colors for styling
    pub colors: ConfirmationPopupColors,
}

/// Footer hints for the confirmation popup
#[derive(Debug, Clone)]
pub struct ConfirmationFooterHints {
    /// Hint for confirm (e.g., "Enter")
    pub confirm: String,
    /// Hint for cancel (e.g., "Esc/x/q")
    pub cancel: String,
}

/// Theme colors for the confirmation popup
#[derive(Debug, Clone)]
pub struct ConfirmationPopupColors {
    #[allow(dead_code)]
    pub title_fg: Color,
    pub target_fg: Color,
    pub instructions_fg: Color,
    pub input_label_fg: Color,
    pub input_fg: Color,
    pub input_bg: Color,
    pub border_fg: Color,
    #[allow(dead_code)]
    pub hint_fg: Color,
    pub error_fg: Color,
}

impl ConfirmationPopupViewModel {
    /// Build view model from confirmation popup state
    pub fn from_state(state: &ConfirmationPopupState, theme: &gh_pr_lander_theme::Theme) -> Self {
        let title = state.title().to_string();
        let target_line = format!("{}: {}", state.action_verb(), state.target_info());
        let instructions = state.instructions().to_string();
        let input_label = "Message:".to_string();
        let input_value = state.input_value.clone();
        let input_is_empty = input_value.is_empty();
        let is_valid = state.is_valid();

        let validation_hint = if !is_valid && state.requires_input() {
            Some("Message is required".to_string())
        } else {
            None
        };

        let footer_hints = ConfirmationFooterHints {
            confirm: "Enter".to_string(),
            cancel: "Esc".to_string(),
        };

        let colors = ConfirmationPopupColors {
            title_fg: theme.accent_primary,
            target_fg: theme.status_info,
            instructions_fg: theme.text_muted,
            input_label_fg: theme.text_primary,
            input_fg: theme.active_fg,
            input_bg: theme.active_bg,
            border_fg: theme.accent_primary,
            hint_fg: theme.text_muted,
            error_fg: theme.status_error,
        };

        Self {
            title,
            target_line,
            instructions,
            input_label,
            input_value,
            input_is_empty,
            is_valid,
            validation_hint,
            footer_hints,
            colors,
        }
    }
}
