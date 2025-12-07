//! Diff content widget for rendering the actual diff.

use crate::highlight::DiffHighlighter;
use crate::model::{DiffLine, FileDiff, LineKind};
use crate::traits::ThemeProvider;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};
use std::collections::HashSet;

/// Pre-computed render data to avoid recomputation per frame.
pub struct DiffRenderData<'a> {
    /// Line number width (from FileDiff cache).
    pub line_no_width: usize,
    /// Comment line numbers for this file (from state cache).
    pub comment_lines: &'a HashSet<u32>,
    /// Display name for the file.
    pub display_name: &'a str,
    /// Total line count (for scroll bounds).
    pub total_lines: usize,
}

/// A single hint entry for the footer.
#[derive(Debug, Clone)]
pub struct FooterHint {
    /// The key (e.g., "c", "R").
    pub key: String,
    /// The description (e.g., "Comment", "Review").
    pub description: String,
}

impl FooterHint {
    /// Create a new footer hint.
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
        }
    }
}

/// Widget for rendering the diff content pane.
pub struct DiffContentWidget<'a, T: ThemeProvider> {
    /// The file diff to render.
    file: Option<&'a FileDiff>,
    /// Pre-computed render data (None if no file).
    render_data: Option<DiffRenderData<'a>>,
    /// Current cursor line.
    cursor_line: usize,
    /// Scroll offset.
    scroll_offset: usize,
    /// Visual selection range (if any).
    visual_selection: Option<(usize, usize)>,
    /// Syntax highlighter.
    highlighter: &'a mut DiffHighlighter,
    /// Theme provider.
    theme: &'a T,
    /// Whether this pane is focused.
    focused: bool,
    /// Footer hints to display at the bottom border.
    footer_hints: Vec<FooterHint>,
}

impl<'a, T: ThemeProvider> DiffContentWidget<'a, T> {
    /// Create a new diff content widget with pre-computed render data.
    pub fn new(
        file: Option<&'a FileDiff>,
        render_data: Option<DiffRenderData<'a>>,
        cursor_line: usize,
        scroll_offset: usize,
        highlighter: &'a mut DiffHighlighter,
        theme: &'a T,
        focused: bool,
    ) -> Self {
        Self {
            file,
            render_data,
            cursor_line,
            scroll_offset,
            visual_selection: None,
            highlighter,
            theme,
            focused,
            footer_hints: Vec::new(),
        }
    }

    /// Set visual selection range.
    pub fn with_selection(mut self, selection: Option<(usize, usize)>) -> Self {
        self.visual_selection = selection;
        self
    }

    /// Set footer hints to display at the bottom border.
    pub fn with_footer_hints(mut self, hints: Vec<FooterHint>) -> Self {
        self.footer_hints = hints;
        self
    }
}

impl<T: ThemeProvider> Widget for DiffContentWidget<'_, T> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        // Draw border
        let border_style = if self.focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title = self
            .render_data
            .as_ref()
            .map(|d| format!(" {} ", d.display_name))
            .unwrap_or_else(|| " No file selected ".to_string());

        // Build footer hints line
        let footer_line = if !self.footer_hints.is_empty() {
            let mut spans = vec![Span::raw(" ")];
            for (i, hint) in self.footer_hints.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::styled(
                        " │ ",
                        Style::default().fg(self.theme.hint_text_foreground()),
                    ));
                }
                spans.push(Span::styled(
                    &hint.key,
                    Style::default()
                        .fg(self.theme.hint_key_foreground())
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!(" {}", hint.description),
                    Style::default().fg(self.theme.hint_text_foreground()),
                ));
            }
            spans.push(Span::raw(" "));
            Some(Line::from(spans))
        } else {
            None
        };

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title);

        if let Some(footer) = footer_line {
            block = block.title_bottom(footer);
        }

        let inner = block.inner(area);
        block.render(area, buf);

        let (Some(file), Some(render_data)) = (self.file, &self.render_data) else {
            // Render empty state
            let msg = "Select a file from the tree";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            buf.set_string(x, y, msg, Style::default().fg(Color::DarkGray));
            return;
        };

        // Use pre-computed values from render_data
        let line_no_width = render_data.line_no_width;
        let comment_lines = render_data.comment_lines;
        let visible_height = inner.height as usize;
        let file_path = file.path.as_str();

        // Render visible lines by iterating directly over hunks
        // This avoids copying the entire display_info vector
        let scroll_end = self.scroll_offset + visible_height;
        let mut current_idx = 0;
        let mut rendered = 0;

        'outer: for hunk in &file.hunks {
            // Hunk header
            if current_idx >= self.scroll_offset && current_idx < scroll_end {
                let y = inner.y + rendered as u16;
                let is_cursor = current_idx == self.cursor_line;
                self.render_hunk_header(&hunk.header, inner.x, y, inner.width, is_cursor, buf);
                rendered += 1;
            }
            current_idx += 1;

            // Skip ahead if we haven't reached scroll offset yet
            if current_idx + hunk.lines.len() <= self.scroll_offset {
                current_idx += hunk.lines.len();
                continue;
            }

            // Render lines
            for line in &hunk.lines {
                if current_idx >= scroll_end {
                    break 'outer;
                }

                if current_idx >= self.scroll_offset {
                    let y = inner.y + rendered as u16;
                    let is_cursor = current_idx == self.cursor_line;
                    let in_selection = self
                        .visual_selection
                        .map(|(start, end)| current_idx >= start && current_idx <= end)
                        .unwrap_or(false);

                    self.render_diff_line(
                        line,
                        inner.x,
                        y,
                        inner.width,
                        line_no_width,
                        is_cursor,
                        in_selection,
                        file_path,
                        comment_lines,
                        buf,
                    );
                    rendered += 1;
                }
                current_idx += 1;
            }
        }
    }
}

impl<T: ThemeProvider> DiffContentWidget<'_, T> {
    fn render_hunk_header(
        &self,
        header: &str,
        x: u16,
        y: u16,
        width: u16,
        is_cursor: bool,
        buf: &mut Buffer,
    ) {
        let (fg, bg) = if is_cursor {
            (
                self.theme.cursor_foreground(),
                self.theme.cursor_background(),
            )
        } else {
            (
                self.theme.hunk_header_foreground(),
                self.theme.hunk_header_background(),
            )
        };

        let style = Style::default().fg(fg).bg(bg);

        // Fill background
        for i in 0..width {
            buf.set_string(x + i, y, " ", style);
        }

        // Render header text (truncate if needed)
        let display_header = if header.len() > width as usize {
            &header[..width as usize]
        } else {
            header
        };
        buf.set_string(x, y, display_header, style);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_diff_line(
        &mut self,
        line: &DiffLine,
        x: u16,
        y: u16,
        width: u16,
        line_no_width: usize,
        is_cursor: bool,
        in_selection: bool,
        file_path: &str,
        comment_lines: &HashSet<u32>,
        buf: &mut Buffer,
    ) {
        // Determine background and foreground colors
        let (fg, bg) = if is_cursor {
            (
                Some(self.theme.cursor_foreground()),
                self.theme.cursor_background(),
            )
        } else if in_selection {
            (None, Color::Rgb(60, 60, 80)) // Selection highlight
        } else {
            let bg = match line.kind {
                LineKind::Addition => self.theme.addition_background(),
                LineKind::Deletion => self.theme.deletion_background(),
                LineKind::Expansion => self.theme.expansion_marker_background(),
                _ => self.theme.context_background(),
            };
            (None, bg)
        };

        let base_style = if let Some(fg) = fg {
            Style::default().fg(fg).bg(bg)
        } else {
            Style::default().bg(bg)
        };

        // Fill background
        for i in 0..width {
            buf.set_string(x + i, y, " ", base_style);
        }

        let mut current_x = x;

        // Line number style - use cursor foreground on cursor line for contrast
        let line_no_style = if is_cursor {
            base_style
        } else {
            base_style.fg(self.theme.line_number_foreground())
        };

        // Render old line number
        let old_no = line
            .old_line
            .map(|n| format!("{:>width$}", n, width = line_no_width))
            .unwrap_or_else(|| " ".repeat(line_no_width));
        buf.set_string(current_x, y, &old_no, line_no_style);
        current_x += line_no_width as u16;

        // Separator
        buf.set_string(current_x, y, " ", base_style);
        current_x += 1;

        // Render new line number
        let new_no = line
            .new_line
            .map(|n| format!("{:>width$}", n, width = line_no_width))
            .unwrap_or_else(|| " ".repeat(line_no_width));
        buf.set_string(current_x, y, &new_no, line_no_style);
        current_x += line_no_width as u16;

        // Separator and prefix
        buf.set_string(current_x, y, " ", base_style);
        current_x += 1;

        let prefix = match line.kind {
            LineKind::Addition => "+",
            LineKind::Deletion => "-",
            LineKind::Expansion => "~",
            _ => " ",
        };
        // Prefix style - use cursor foreground on cursor line, otherwise semantic colors
        let prefix_style = if is_cursor {
            base_style
        } else {
            match line.kind {
                LineKind::Addition => base_style.fg(Color::Green),
                LineKind::Deletion => base_style.fg(Color::Red),
                LineKind::Expansion => base_style.fg(self.theme.expansion_marker_foreground()),
                _ => base_style,
            }
        };
        buf.set_string(current_x, y, prefix, prefix_style);
        current_x += 1;

        // Content area width
        let content_width = width.saturating_sub(current_x - x) as usize;

        // Render content (with syntax highlighting for non-expansion lines)
        if line.kind == LineKind::Expansion {
            // Expansion marker text
            let text = "... expand to see more ...";
            buf.set_string(
                current_x,
                y,
                text,
                base_style.fg(self.theme.expansion_marker_foreground()),
            );
        } else {
            // Syntax highlight and render
            let highlighted = self.highlighter.highlight_line(file_path, &line.content);

            let mut col = 0;
            for span in highlighted {
                if col >= content_width {
                    break;
                }

                let available = content_width - col;
                let text = if span.text.len() > available {
                    &span.text[..available]
                } else {
                    &span.text
                };

                let mut style = base_style;
                // Only apply syntax highlighting colors when not on cursor line
                // to maintain proper contrast
                if !is_cursor {
                    if let Some(fg) = span.fg {
                        style = style.fg(fg);
                    }
                }
                if span.bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if span.italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }

                buf.set_string(current_x + col as u16, y, text, style);
                col += text.len();
            }
        }

        // Show expanded indicator
        if line.is_expanded {
            let indicator_x = x + width - 2;
            if indicator_x > current_x {
                buf.set_string(indicator_x, y, "↕", base_style.fg(Color::DarkGray));
            }
        }

        // Show comment indicator (O(1) HashSet lookup instead of O(n) linear search)
        let line_no = line.new_line.or(line.old_line).unwrap_or(0);
        let has_comment = comment_lines.contains(&line_no);
        if has_comment {
            let indicator_x = x + width - 4;
            if indicator_x > current_x {
                buf.set_string(
                    indicator_x,
                    y,
                    "💬",
                    base_style.fg(self.theme.comment_indicator_foreground()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::DiffHighlighter;
    use crate::model::{DiffLine, FileDiff, Hunk};
    use crate::traits::DefaultTheme;

    #[test]
    fn test_diff_content_widget_creation() {
        let mut file = FileDiff::new("src/test.rs");
        let mut hunk = Hunk::new(1, 3, 1, 4);
        hunk.lines.push(DiffLine::context("line 1", 1, 1));
        hunk.lines.push(DiffLine::addition("new line", 2));
        file.hunks.push(hunk);

        // Get cached render data (no large copies needed)
        let line_no_width = file.line_no_width();
        let display_name = file.display_name().to_string();
        let total_lines = file.total_lines();
        let comment_lines = HashSet::new();

        let render_data = DiffRenderData {
            line_no_width,
            comment_lines: &comment_lines,
            display_name: &display_name,
            total_lines,
        };

        let mut highlighter = DiffHighlighter::new();
        let theme = DefaultTheme;

        let _widget = DiffContentWidget::new(
            Some(&file),
            Some(render_data),
            0,
            0,
            &mut highlighter,
            &theme,
            true,
        );
    }
}
