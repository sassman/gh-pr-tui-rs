use ratatui::style::Color;

/// View model for debug console - all presentation data pre-computed
#[derive(Debug, Clone)]
pub struct DebugConsoleViewModel {
    /// Pre-formatted title with log count and mode
    pub title: String,
    /// Pre-formatted footer text
    pub footer: String,
    /// Visible log lines for current viewport (scroll_offset applied)
    pub visible_logs: Vec<LogLine>,
}

/// A single log line with pre-formatted text and color
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Pre-formatted text: "12:34:56.789 ERROR my_module          message text"
    pub text: String,
    /// Pre-determined color based on log level
    pub color: Color,
}

impl DebugConsoleViewModel {
    /// Build view model from debug console state
    /// Returns only the visible logs for the current viewport
    pub fn from_state(
        logs: &[crate::log_capture::LogEntry],
        scroll_offset: usize,
        auto_scroll: bool,
        visible_height: usize,
        theme: &crate::theme::Theme,
    ) -> Self {
        use ::log::Level;

        let log_count = logs.len();

        // Calculate viewport range
        let actual_scroll = if auto_scroll {
            // Auto-scroll: show most recent logs
            log_count.saturating_sub(visible_height)
        } else {
            // Manual scroll: use offset, clamped to valid range
            scroll_offset.min(log_count.saturating_sub(visible_height))
        };

        // Build pre-formatted log lines for visible viewport only
        let visible_logs: Vec<LogLine> = logs
            .iter()
            .skip(actual_scroll)
            .take(visible_height)
            .map(|entry| {
                // Determine color based on log level
                let color = match entry.level {
                    Level::Error => theme.status_error,
                    Level::Warn => theme.status_warning,
                    Level::Info => theme.text_primary,
                    Level::Debug => theme.text_secondary,
                    Level::Trace => theme.text_muted,
                };

                // Format timestamp
                let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");

                // Format level (5 chars fixed width)
                let level_str = format!("{:5}", entry.level.to_string().to_uppercase());

                // Truncate or pad target to 20 chars
                let target_short = if entry.target.len() > 20 {
                    format!("{}...", &entry.target[..17])
                } else {
                    format!("{:20}", entry.target)
                };

                // Pre-format the entire line
                let text = format!(
                    "{} {} {} {}",
                    timestamp, level_str, target_short, entry.message
                );

                LogLine { text, color }
            })
            .collect();

        // Pre-format title
        let mode_text = if auto_scroll { "[AUTO]" } else { "[MANUAL]" };
        let title = format!(" Debug Console ({}) {} ", log_count, mode_text);

        // Pre-format footer
        let footer = " `~` Close | j/k Scroll | a Auto-scroll | c Clear ".to_string();

        Self {
            title,
            footer,
            visible_logs,
        }
    }
}
