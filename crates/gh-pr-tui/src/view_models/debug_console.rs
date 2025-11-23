use ratatui::style::Color;

/// View model for debug console - all presentation data pre-computed
#[derive(Debug, Clone)]
pub struct DebugConsoleViewModel {
    /// Pre-formatted title with log count and mode
    pub title: String,
    /// Pre-formatted footer text
    pub footer: String,
    /// Pre-formatted visible log lines
    pub visible_logs: Vec<LogLine>,
    /// Current scroll offset (for informational purposes)
    pub scroll_offset: usize,
    /// Visible height (for page down calculations)
    pub visible_height: usize,
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
    pub fn from_state(
        logs: &[crate::log_capture::LogEntry],
        scroll_offset: usize,
        auto_scroll: bool,
        console_height: usize,
        theme: &crate::theme::Theme,
    ) -> Self {
        use ::log::Level;

        let log_count = logs.len();

        // Calculate visible range
        // Subtract 2 for top and bottom borders (title and footer are inside borders)
        let visible_height = console_height.saturating_sub(2);
        let total_logs = logs.len();

        let actual_scroll_offset = if auto_scroll {
            // Auto-scroll: show most recent logs
            total_logs.saturating_sub(visible_height)
        } else {
            // Manual scroll: use provided scroll_offset
            scroll_offset.min(total_logs.saturating_sub(visible_height))
        };

        // Build pre-formatted log lines
        let visible_logs: Vec<LogLine> = logs
            .iter()
            .skip(actual_scroll_offset)
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
        let title = format!(
            " Debug Console ({}/{}) {} ",
            actual_scroll_offset + visible_height.min(total_logs),
            log_count,
            mode_text
        );

        // Pre-format footer
        let footer = " `~` Close | j/k Scroll | a Auto-scroll | c Clear ".to_string();

        Self {
            title,
            footer,
            visible_logs,
            scroll_offset: actual_scroll_offset,
            visible_height,
        }
    }
}
