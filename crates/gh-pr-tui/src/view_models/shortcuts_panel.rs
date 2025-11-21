use ratatui::text::{Line, Span};

/// View model for shortcuts help panel - all presentation data pre-computed
#[derive(Debug, Clone)]
pub struct ShortcutsPanelViewModel {
    /// Title with scroll indicator
    pub title: String,
    /// All text lines ready for rendering (with colors applied)
    pub content_lines: Vec<Line<'static>>,
    /// Actual scroll offset (clamped to valid range)
    pub scroll_offset: usize,
    /// Maximum scroll offset
    pub max_scroll: usize,
    /// Footer text
    pub footer_line: Line<'static>,
}

impl ShortcutsPanelViewModel {
    /// Build view model from shortcuts and scroll state
    pub fn from_state(
        shortcuts: Vec<crate::shortcuts::ShortcutCategory>,
        scroll_offset: usize,
        visible_height: usize,
        theme: &crate::theme::Theme,
    ) -> Self {
        // Build content lines with formatting
        let mut content_lines = Vec::new();

        for category in shortcuts {
            // Category header
            content_lines.push(Line::from(vec![Span::styled(
                category.name,
                ratatui::style::Style::default()
                    .fg(theme.status_warning)
                    .add_modifier(ratatui::style::Modifier::BOLD | ratatui::style::Modifier::UNDERLINED),
            )]));
            content_lines.push(Line::from(""));

            // Items in this category
            for shortcut in category.shortcuts {
                content_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:18}", shortcut.key_display),
                        ratatui::style::Style::default()
                            .fg(theme.status_success)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                    Span::styled(
                        shortcut.description,
                        ratatui::style::Style::default().fg(theme.text_secondary),
                    ),
                ]));
            }

            content_lines.push(Line::from(""));
        }

        // Calculate scroll info
        let total_lines = content_lines.len();
        let max_scroll = total_lines.saturating_sub(visible_height);
        let actual_scroll = scroll_offset.min(max_scroll);

        // Format title with scroll indicator
        let title = if total_lines > visible_height {
            format!(
                " Keyboard Shortcuts  [{}/{}] ",
                actual_scroll + 1,
                total_lines
            )
        } else {
            " Keyboard Shortcuts ".to_string()
        };

        // Pre-format footer
        let footer_line = Line::from(vec![
            Span::styled("Press ", ratatui::style::Style::default().fg(theme.text_muted)),
            Span::styled(
                "x",
                ratatui::style::Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(" or ", ratatui::style::Style::default().fg(theme.text_muted)),
            Span::styled(
                "Esc",
                ratatui::style::Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(" to close this help", ratatui::style::Style::default().fg(theme.text_muted)),
        ]);

        Self {
            title,
            content_lines,
            scroll_offset: actual_scroll,
            max_scroll,
            footer_line,
        }
    }
}
