use ratatui::{
    prelude::*,
    widgets::*,
};

/// Represents a single line in the log with metadata
#[derive(Debug, Clone)]
pub struct LogLine {
    pub content: String,
    pub timestamp: String,
    /// True if this line is part of an error section
    pub is_error: bool,
    pub is_warning: bool,
    pub is_header: bool,
    pub error_level: ErrorLevel,
    /// The build step this line belongs to (for context)
    pub step_name: String,
    /// True if this line starts an error section (contains "error:")
    pub is_error_start: bool,
    /// Styled segments from ANSI parsing (if available from new parser)
    pub styled_segments: Vec<gh_actions_log_parser::StyledSegment>,
    /// Workflow command if this line contains one
    pub command: Option<gh_actions_log_parser::WorkflowCommand>,
    /// Group nesting level (0 = not in group)
    pub group_level: usize,
    /// Title of containing group
    pub group_title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorLevel {
    None,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone)]
pub struct LogPanel {
    /// All log lines in a flat unified view
    pub lines: Vec<LogLine>,
    /// Indices of lines that contain errors (for fast n/p navigation)
    pub error_indices: Vec<usize>,
    /// Current error index we're focused on (index into error_indices)
    pub current_error_idx: usize,
    /// Scroll offset (line number at top of viewport)
    pub scroll_offset: usize,
    pub horizontal_scroll: usize,
    pub pr_context: PrContext,
    pub show_timestamps: bool,
}

#[derive(Debug, Clone)]
pub struct PrContext {
    pub number: usize,
    pub title: String,
    pub author: String,
}

/// Legacy structure for backward compatibility with task.rs
#[derive(Debug, Clone)]
pub struct LogSection {
    pub step_name: String,
    pub error_lines: Vec<String>,
    pub has_extracted_errors: bool,
}

/// Extract error context from build logs
/// Returns lines around errors (±5 lines before and after)
/// Returns empty vector if no meaningful errors found (user will see full log instead)
pub fn extract_error_context(log_text: &str, _step_name: &str) -> Vec<String> {
    let lines: Vec<&str> = log_text.lines().collect();
    let mut error_indices = Vec::new();

    // Find lines that contain error indicators
    // Prioritize lines that START with "error" for build/compilation errors
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        // Skip lines that are clearly not errors (comments)
        if lower.starts_with("# error") || lower.starts_with("// error") {
            continue;
        }

        // PRIORITY 1: Lines that start with "error" (typical build errors)
        if lower.starts_with("error:")
            || lower.starts_with("error[")
            || lower.starts_with("error ")
        {
            error_indices.push(idx);
            continue;
        }

        // PRIORITY 2: Lines that start with other error indicators
        if lower.starts_with("failed:")
            || lower.starts_with("failure:")
            || lower.starts_with("fatal:")
        {
            error_indices.push(idx);
            continue;
        }

        // PRIORITY 3: Lines containing error in context (less reliable)
        if lower.contains("error:")
            || lower.contains("failed:")
            || lower.contains("✗")
            || lower.contains("❌")
            || (lower.contains("error") && (lower.contains("line") || lower.contains("at ")))
        {
            error_indices.push(idx);
        }
    }

    // Only return error context if we found at least 2 error lines
    if error_indices.len() < 2 {
        return Vec::new();
    }

    // For each error, extract context (±5 lines)
    let mut result = Vec::new();
    let mut covered_ranges = Vec::new();

    for (idx, &error_idx) in error_indices.iter().enumerate() {
        let start = error_idx.saturating_sub(5);
        let end = (error_idx + 10).min(lines.len()); // Keep 10 lines after for context

        // Check if this range overlaps with already covered ranges
        let mut should_add = true;
        for &(covered_start, covered_end) in &covered_ranges {
            if start <= covered_end && end >= covered_start {
                should_add = false;
                break;
            }
        }

        if should_add {
            covered_ranges.push((start, end));

            // Add skip indicator if we're not at the beginning and this is the first error
            if idx == 0 && start > 0 {
                result.push(format!("... [skipped {} lines] ...", start));
                result.push("".to_string());
            }

            for line in lines.iter().take(end).skip(start) {
                result.push(line.to_string());
            }

            // Add separator between different error contexts
            if idx < error_indices.len() - 1 {
                result.push("".to_string());
                result.push("─".repeat(80));
                result.push("".to_string());
            }
        }
    }

    result
}

/// Render the log panel as a card overlay with PR context header
/// Takes the available area (excluding top tabs and bottom panels)
pub fn render_log_panel_card(f: &mut Frame, panel: &LogPanel, theme: &crate::theme::Theme, available_area: Rect) {
    // Use Clear widget to completely clear the underlying content
    f.render_widget(Clear, available_area);

    // Then render a solid background to ensure complete coverage
    let background = Block::default()
        .style(Style::default().bg(theme.bg_panel));
    f.render_widget(background, available_area);

    // Use the full available area (same dimensions as PR panel)
    let card_area = available_area;

    // Split card into PR header (3 lines) and log content
    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // PR context header
            Constraint::Min(0),     // Log content
        ])
        .split(card_area);

    // Render PR context header
    let pr_header_text = vec![
        Line::from(vec![
            Span::styled(
                format!("#{} ", panel.pr_context.number),
                Style::default()
                    .fg(theme.status_info)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                panel.pr_context.title.clone(),
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("by {}", panel.pr_context.author),
            Style::default().fg(theme.text_muted),
        )),
    ];

    let pr_header = Paragraph::new(pr_header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(theme.bg_panel)),
    );

    f.render_widget(pr_header, card_chunks[0]);

    // Render log content in the remaining area
    render_log_panel_content(f, panel, card_chunks[1], theme);
}

/// Render the log panel showing build failure logs using a Table widget
/// OPTIMIZED: Only renders visible lines (viewport-based rendering)
fn render_log_panel_content(f: &mut Frame, panel: &LogPanel, area: Rect, theme: &crate::theme::Theme) {
    let visible_height = area.height.saturating_sub(2) as usize; // -2 for borders

    // Calculate viewport (only render visible lines)
    let total_lines = panel.lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let start_line = panel.scroll_offset.min(max_scroll);
    let end_line = (start_line + visible_height).min(total_lines);

    // Get current error line index (for highlighting)
    let current_error_line = panel.error_indices.get(panel.current_error_idx).copied();

    // Get current step name for context (from first visible line or current error)
    let current_step = if let Some(error_line_idx) = current_error_line {
        panel.lines.get(error_line_idx).map(|l| l.step_name.as_str())
    } else {
        panel.lines.get(start_line).map(|l| l.step_name.as_str())
    }.unwrap_or("Unknown");

    // Build table rows ONLY for visible lines (PERFORMANCE OPTIMIZATION)
    let rows: Vec<Row> = panel.lines[start_line..end_line]
        .iter()
        .enumerate()
        .map(|(viewport_idx, log_line)| {
            let actual_line_idx = start_line + viewport_idx;
            let is_current_error = Some(actual_line_idx) == current_error_line;

            // Apply horizontal scrolling to content
            // Add error marker for error section start lines
            let visible_content = if panel.horizontal_scroll > 0 {
                let content = if log_line.is_error_start {
                    format!("▶ {}", log_line.content)
                } else {
                    log_line.content.clone()
                };
                content.chars()
                    .skip(panel.horizontal_scroll)
                    .collect::<String>()
            } else if log_line.is_error_start {
                format!("▶ {}", log_line.content)
            } else {
                log_line.content.clone()
            };

            // Determine style based on line metadata
            let mut style = if log_line.is_header {
                // Section headers - bright cyan
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
            } else if log_line.is_error_start {
                // Error section start (line with "error:") - bright red with bold
                Style::default()
                    .fg(theme.status_error)
                    .add_modifier(Modifier::BOLD)
            } else if log_line.is_error {
                // Error section continuation lines - red (no bold)
                Style::default().fg(theme.status_error)
            } else if log_line.is_warning {
                // Warnings - yellow
                Style::default().fg(theme.status_warning)
            } else {
                // Normal lines - light slate
                Style::default().fg(theme.text_primary)
            };

            // Highlight current error with background
            if is_current_error {
                style = style.add_modifier(Modifier::REVERSED);
            }

            // Add background color to prevent bleed-through
            style = style.bg(theme.bg_panel);

            // Create cells based on timestamp visibility
            if panel.show_timestamps {
                Row::new(vec![
                    Cell::from(log_line.timestamp.clone()).style(
                        Style::default()
                            .fg(theme.text_muted)
                            .bg(theme.bg_panel)
                    ),
                    Cell::from(visible_content).style(style),
                ])
            } else {
                // When timestamps hidden, use single column
                Row::new(vec![Cell::from(visible_content).style(style)])
            }
        })
        .collect();

    // Build scroll info with error navigation and step context
    let scroll_info = if !panel.error_indices.is_empty() {
        format!(
            " Build Logs [Line {}/{}] | Step: {} | Error {}/{} | n: next, p: prev, h/l: scroll H, j/k: scroll V, t: timestamps, x: close ",
            start_line + 1,
            total_lines,
            current_step,
            panel.current_error_idx + 1,
            panel.error_indices.len()
        )
    } else {
        format!(
            " Build Logs [Line {}/{}] | Step: {} | No errors | h/l: scroll H, j/k: scroll V, t: timestamps, x: close ",
            start_line + 1,
            total_lines,
            current_step
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(scroll_info)
        .border_style(
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg_panel));

    // Configure table with or without timestamp column
    let widths = if panel.show_timestamps {
        vec![Constraint::Length(30), Constraint::Min(0)]
    } else {
        vec![Constraint::Percentage(100)]
    };

    let table = Table::new(rows, widths)
        .block(block)
        .column_spacing(0) // No spacing between columns to prevent gaps
        .style(
            Style::default()
                .fg(theme.text_primary)
                .bg(theme.bg_panel),
        );

    f.render_widget(table, area);
}

/// Extract timestamp from log line if present
/// Returns (timestamp, content) tuple
fn extract_timestamp(line: &str) -> (String, String) {
    // GitHub Actions logs format: "2024-01-15T10:30:00.1234567Z some log line"
    if line.len() > 30 {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() > 30
            && chars[4] == '-'
            && chars[7] == '-'
            && chars[10] == 'T'
            && chars[13] == ':'
            && chars[16] == ':'
            && (chars[19] == '.' || chars[19] == 'Z')
        {
            // Find where timestamp ends (look for 'Z' followed by space)
            if let Some(pos) = line.find("Z ") {
                let timestamp = line[..pos + 1].to_string(); // Include the 'Z'
                let content = line[pos + 2..].to_string(); // Skip "Z " to get content
                return (timestamp, content);
            }
        }
    }
    // No timestamp found
    (String::new(), line.to_string())
}

/// Convert legacy LogSections into unified LogPanel format
/// This flattens all sections into a single view with error highlighting
/// Error sections start with "error:" and end with an empty line
pub fn create_log_panel_from_sections(
    log_sections: Vec<LogSection>,
    pr_context: PrContext,
) -> LogPanel {
    let mut lines = Vec::new();
    let mut error_indices = Vec::new();

    for (section_idx, section) in log_sections.iter().enumerate() {
        let step_name = section.step_name.clone();

        // Add section header
        let header = format!("━━━ {} ━━━", step_name);
        lines.push(LogLine {
            content: header,
            timestamp: String::new(),
            is_error: false,
            is_warning: false,
            is_header: true,
            error_level: ErrorLevel::None,
            step_name: step_name.clone(),
            is_error_start: false,
            styled_segments: Vec::new(),
            command: None,
            group_level: 0,
            group_title: None,
        });

        // Track if we're inside an error section
        let mut in_error_section = false;

        // Add all lines from this section
        for line in &section.error_lines {
            let (timestamp, content) = extract_timestamp(line);
            let content_lower = content.to_lowercase();

            // Check if this line starts an error section
            let starts_error = content_lower.contains("error:");

            // Check if this is an empty line (ends error section)
            let is_empty = content.trim().is_empty();

            // State machine for error sections
            if starts_error && !in_error_section {
                // Start of new error section
                in_error_section = true;
                error_indices.push(lines.len()); // Store start index of error section
            }

            let is_in_error_section = in_error_section;

            // Add the line
            lines.push(LogLine {
                content,
                timestamp,
                is_error: is_in_error_section,
                is_warning: false,
                is_header: false,
                error_level: if is_in_error_section { ErrorLevel::Error } else { ErrorLevel::None },
                step_name: step_name.clone(),
                is_error_start: starts_error,
                styled_segments: Vec::new(),
                command: None,
                group_level: 0,
                group_title: None,
            });

            // End error section on empty line
            if is_empty && in_error_section {
                in_error_section = false;
            }
        }

        // Add separator between sections (except after last section)
        if section_idx < log_sections.len() - 1 {
            lines.push(LogLine {
                content: "─".repeat(80),
                timestamp: String::new(),
                is_error: false,
                is_warning: false,
                is_header: false,
                error_level: ErrorLevel::None,
                step_name: step_name.clone(),
                is_error_start: false,
                styled_segments: Vec::new(),
                command: None,
                group_level: 0,
                group_title: None,
            });
        }
    }

    LogPanel {
        lines,
        error_indices,
        current_error_idx: 0,
        scroll_offset: 0,
        horizontal_scroll: 0,
        pr_context,
        show_timestamps: false,
    }
}

impl LogPanel {
    /// Navigate to the next error
    pub fn next_error(&mut self) {
        if self.error_indices.is_empty() {
            return;
        }

        // Move to next error
        if self.current_error_idx < self.error_indices.len() - 1 {
            self.current_error_idx += 1;
        } else {
            // Wrap around to first error
            self.current_error_idx = 0;
        }

        // Scroll to make the error visible (centered if possible)
        if let Some(&line_idx) = self.error_indices.get(self.current_error_idx) {
            self.scroll_offset = line_idx.saturating_sub(5); // Show 5 lines of context above
        }
    }

    /// Navigate to the previous error
    pub fn prev_error(&mut self) {
        if self.error_indices.is_empty() {
            return;
        }

        // Move to previous error
        if self.current_error_idx > 0 {
            self.current_error_idx -= 1;
        } else {
            // Wrap around to last error
            self.current_error_idx = self.error_indices.len() - 1;
        }

        // Scroll to make the error visible (centered if possible)
        if let Some(&line_idx) = self.error_indices.get(self.current_error_idx) {
            self.scroll_offset = line_idx.saturating_sub(5); // Show 5 lines of context above
        }
    }
}
