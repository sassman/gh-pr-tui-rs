//! Main diff viewer composite widget.

use super::{DiffContentWidget, DiffRenderData, FileTreeWidget, FooterHint, ReviewPopupWidget};
use crate::highlight::DiffHighlighter;
use crate::state::DiffViewerState;
use crate::traits::ThemeProvider;
use ratatui::prelude::*;
use ratatui::widgets::Widget;
use std::collections::HashSet;

/// The main diff viewer widget.
///
/// This is a composite widget that combines:
/// - File tree navigation (left pane)
/// - Diff content (right pane)
/// - Optional popups (review submission, comment editor)
///
/// # Example
///
/// ```ignore
/// use gh_diff_viewer::{DiffViewer, DiffViewerState, DiffHighlighter};
/// use gh_diff_viewer::traits::DefaultTheme;
///
/// let mut highlighter = DiffHighlighter::new();
/// let theme = DefaultTheme;
///
/// let widget = DiffViewer::new(&mut highlighter, &theme);
/// frame.render_stateful_widget(widget, area, &mut state);
/// ```
pub struct DiffViewer<'a, T: ThemeProvider> {
    /// Syntax highlighter.
    highlighter: &'a mut DiffHighlighter,
    /// Theme provider.
    theme: &'a T,
    /// Footer hints to display.
    footer_hints: Vec<FooterHint>,
}

impl<'a, T: ThemeProvider> DiffViewer<'a, T> {
    /// Create a new diff viewer widget.
    pub fn new(highlighter: &'a mut DiffHighlighter, theme: &'a T) -> Self {
        Self {
            highlighter,
            theme,
            footer_hints: Vec::new(),
        }
    }

    /// Set footer hints to display in the diff content pane.
    pub fn with_footer_hints(mut self, hints: Vec<FooterHint>) -> Self {
        self.footer_hints = hints;
        self
    }
}

impl<T: ThemeProvider> Widget for DiffViewer<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // This widget requires state - use render_ref with state instead
        // For the Widget trait, we just render a placeholder
        buf.set_string(
            area.x,
            area.y,
            "Use DiffViewer with render_stateful_widget",
            Style::default().fg(Color::Red),
        );
    }
}

/// Stateful rendering for DiffViewer.
impl<T: ThemeProvider> DiffViewer<'_, T> {
    /// Render the diff viewer with state.
    pub fn render_with_state(self, area: Rect, buf: &mut Buffer, state: &mut DiffViewerState) {
        // Update viewport height from actual render area (for scroll calculations)
        // Subtract 2 for borders
        state.viewport_height = area.height.saturating_sub(2) as usize;

        // Calculate layout
        let chunks = if state.nav.show_file_tree {
            Layout::horizontal([Constraint::Percentage(22), Constraint::Percentage(78)]).split(area)
        } else {
            Layout::horizontal([Constraint::Length(0), Constraint::Percentage(100)]).split(area)
        };

        // Extract navigation state first to avoid borrow issues
        let file_tree_focused = state.nav.file_tree_focused;
        let cursor_line = state.nav.cursor_line;
        let file_tree_cursor = state.nav.file_tree_cursor;
        let scroll_offset = state.nav.scroll_offset;
        let visual_selection = state.nav.visual_selection();

        // Render file tree (left pane) using cached flat entries
        if state.nav.show_file_tree {
            // Get flat entries (populates cache if needed)
            let flat_entries = state.flat_tree().to_vec();
            let file_tree = FileTreeWidget::new(
                &flat_entries,
                file_tree_cursor,
                file_tree_focused,
                self.theme,
            );
            file_tree.render(chunks[0], buf);
        }

        // Prepare cached render data for diff content
        let file_path = state.current_file().map(|f| f.path.clone());
        let comment_lines: HashSet<u32> = file_path
            .as_ref()
            .map(|p| state.comment_lines_for_file(p))
            .unwrap_or_default();

        // Get cached values from current file (only small values, no large copies)
        let render_data = if let Some(file) = state.current_file_mut() {
            let line_no_width = file.line_no_width();
            let display_name = file.display_name().to_string();
            let total_lines = file.total_lines();
            Some((line_no_width, display_name, total_lines))
        } else {
            None
        };

        // Get immutable reference for widget
        let current_file = state.current_file();

        // Build DiffRenderData with owned data (no large vector copies)
        let render_data_struct = render_data.as_ref().map(
            |(line_no_width, display_name, total_lines)| DiffRenderData {
                line_no_width: *line_no_width,
                comment_lines: &comment_lines,
                display_name,
                total_lines: *total_lines,
            },
        );

        let diff_content = DiffContentWidget::new(
            current_file,
            render_data_struct,
            if file_tree_focused { 0 } else { cursor_line },
            scroll_offset,
            self.highlighter,
            self.theme,
            !file_tree_focused,
        )
        .with_selection(visual_selection)
        .with_footer_hints(self.footer_hints.clone());

        diff_content.render(chunks[1], buf);

        // Render review popup if visible
        if state.show_review_popup {
            let popup = ReviewPopupWidget::new(
                state.pending_comments.len(),
                state.selected_review_event,
                self.theme,
            );
            popup.render(area, buf);
        }

        // TODO: Render comment editor popup if active
        if state.comment_editor.is_some() {
            // Comment editor popup would go here
            self.render_comment_editor(area, buf, state);
        }
    }

    fn render_comment_editor(&self, area: Rect, buf: &mut Buffer, state: &DiffViewerState) {
        let Some(ref editor) = state.comment_editor else {
            return;
        };

        use ratatui::widgets::{Block, Borders, Clear};

        // Calculate popup dimensions
        let popup_width = 60.min(area.width.saturating_sub(4));
        let popup_height = 10.min(area.height.saturating_sub(4));

        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear area
        Clear.render(popup_area, buf);

        // Draw border
        let title = format!(" Comment on line {} ", editor.position.line);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(title);

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Render the comment body with cursor
        let lines: Vec<&str> = if editor.body.is_empty() {
            vec![""]
        } else {
            editor.body.lines().collect()
        };

        for (i, line) in lines.iter().take(inner.height as usize - 1).enumerate() {
            buf.set_string(inner.x, inner.y + i as u16, line, Style::default());
        }

        // Show cursor (simple implementation)
        let cursor_line = editor.current_line();
        let cursor_col = editor.current_column();
        if cursor_line < inner.height as usize && cursor_col < inner.width as usize {
            let cursor_x = inner.x + cursor_col as u16;
            let cursor_y = inner.y + cursor_line as u16;
            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height - 1 {
                buf.set_style(
                    Rect::new(cursor_x, cursor_y, 1, 1),
                    Style::default().bg(Color::White).fg(Color::Black),
                );
            }
        }

        // Render hints
        let hints = "Ctrl+Enter: Submit | Esc: Cancel";
        let hint_y = inner.y + inner.height - 1;
        buf.set_string(inner.x, hint_y, hints, Style::default().fg(Color::DarkGray));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiffLine, FileDiff, Hunk, PullRequestDiff};
    use crate::traits::DefaultTheme;

    fn sample_diff() -> PullRequestDiff {
        let mut diff = PullRequestDiff::new("base", "head");
        let mut file = FileDiff::new("src/main.rs");
        let mut hunk = Hunk::new(1, 3, 1, 4);
        hunk.lines.push(DiffLine::context("fn main() {", 1, 1));
        hunk.lines
            .push(DiffLine::addition("    println!(\"Hello\");", 2));
        hunk.lines.push(DiffLine::context("}", 2, 3));
        file.hunks.push(hunk);
        file.recalculate_stats();
        diff.files.push(file);
        diff.recalculate_totals();
        diff
    }

    #[test]
    fn test_diff_viewer_creation() {
        let mut highlighter = DiffHighlighter::new();
        let theme = DefaultTheme;
        let _widget = DiffViewer::new(&mut highlighter, &theme);
    }

    #[test]
    fn test_diff_viewer_render() {
        let mut highlighter = DiffHighlighter::new();
        let theme = DefaultTheme;
        let widget = DiffViewer::new(&mut highlighter, &theme);

        let mut state = DiffViewerState::new(sample_diff());
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));

        widget.render_with_state(Rect::new(0, 0, 80, 24), &mut buf, &mut state);

        // Buffer should have content
        // (Just checking it doesn't panic)
    }
}
