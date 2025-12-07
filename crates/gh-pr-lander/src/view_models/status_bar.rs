//! Status Bar View Model
//!
//! Pre-computes presentation data for the status bar.

use crate::state::{AppState, StatusKind};
use ratatui::style::{Color, Modifier, Style};

/// View model for rendering the status bar
#[derive(Debug, Clone)]
pub struct StatusBarViewModel {
    /// Whether there's a message to show
    pub has_message: bool,
    /// Emoji/icon for the status
    pub emoji: &'static str,
    /// The message text
    pub message: String,
    /// Timestamp formatted for display (e.g., "14:32:05")
    pub timestamp: String,
    /// Source action for context
    pub source: String,
    /// Foreground style (color based on kind)
    pub message_style: Style,
    /// Background color for the bar
    pub bg_color: Color,
    /// Style for metadata (timestamp, source)
    pub metadata_style: Style,
}

impl StatusBarViewModel {
    pub fn from_state(state: &AppState) -> Self {
        let theme = &state.theme;

        if let Some(msg) = state.status_bar.latest() {
            let fg_color = match msg.kind {
                StatusKind::Running => theme.status_warning,
                StatusKind::Success => theme.status_success,
                StatusKind::Error => theme.status_error,
                StatusKind::Warning => theme.status_warning,
                StatusKind::Info => theme.status_info,
            };

            Self {
                has_message: true,
                emoji: msg.kind.emoji(),
                message: msg.message.clone(),
                timestamp: msg.timestamp.format("%H:%M:%S").to_string(),
                source: msg.source_action.clone(),
                message_style: Style::default().fg(fg_color).add_modifier(Modifier::BOLD),
                bg_color: theme.bg_primary,
                metadata_style: Style::default().fg(theme.text_muted),
            }
        } else {
            // Welcome message when no status messages
            Self {
                has_message: true,
                emoji: "👋",
                message: "Welcome to GitHub PR Lander".to_string(),
                timestamp: String::new(),
                source: String::new(),
                message_style: Style::default()
                    .fg(theme.text_muted)
                    .add_modifier(Modifier::ITALIC),
                bg_color: theme.bg_primary,
                metadata_style: Style::default().fg(theme.text_muted),
            }
        }
    }
}
