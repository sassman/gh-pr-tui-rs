//! Navigation state for cursor, scroll, and selection.

use crate::model::DiffSide;

/// Navigation state within the diff viewer.
#[derive(Debug, Clone, Default)]
pub struct NavigationState {
    /// Current file index in the files list.
    pub selected_file: usize,
    /// Cursor line within the current file's diff content (display line, not source line).
    pub cursor_line: usize,
    /// Cursor position in the file tree (separate from diff content cursor).
    pub file_tree_cursor: usize,
    /// Scroll offset (first visible line).
    pub scroll_offset: usize,
    /// Current selection mode.
    pub selection_mode: SelectionMode,
    /// Whether the file tree pane is focused.
    pub file_tree_focused: bool,
    /// Whether the file tree is visible.
    pub show_file_tree: bool,
}

impl NavigationState {
    /// Create new navigation state.
    pub fn new() -> Self {
        Self {
            selected_file: 0,
            cursor_line: 0,
            file_tree_cursor: 0,
            scroll_offset: 0,
            selection_mode: SelectionMode::Normal,
            file_tree_focused: true,
            show_file_tree: true,
        }
    }

    /// Move cursor down by one line.
    pub fn cursor_down(&mut self, max_lines: usize) {
        if self.cursor_line + 1 < max_lines {
            self.cursor_line += 1;
        }
    }

    /// Move cursor up by one line.
    pub fn cursor_up(&mut self) {
        self.cursor_line = self.cursor_line.saturating_sub(1);
    }

    /// Move cursor to the first line.
    pub fn cursor_first(&mut self) {
        self.cursor_line = 0;
        self.scroll_offset = 0;
    }

    /// Move cursor to the last line.
    pub fn cursor_last(&mut self, max_lines: usize) {
        self.cursor_line = max_lines.saturating_sub(1);
    }

    /// Move to the next file.
    pub fn next_file(&mut self, file_count: usize) {
        if self.selected_file + 1 < file_count {
            self.selected_file += 1;
            self.cursor_line = 0;
            self.scroll_offset = 0;
        }
    }

    /// Move to the previous file.
    pub fn prev_file(&mut self) {
        if self.selected_file > 0 {
            self.selected_file -= 1;
            self.cursor_line = 0;
            self.scroll_offset = 0;
        }
    }

    /// Select a specific file by index.
    pub fn select_file(&mut self, index: usize, file_count: usize) {
        if index < file_count {
            self.selected_file = index;
            self.cursor_line = 0;
            self.scroll_offset = 0;
        }
    }

    /// Toggle focus between file tree and diff content.
    pub fn toggle_focus(&mut self) {
        self.file_tree_focused = !self.file_tree_focused;
    }

    /// Toggle file tree visibility.
    pub fn toggle_file_tree(&mut self) {
        self.show_file_tree = !self.show_file_tree;
        if !self.show_file_tree {
            self.file_tree_focused = false;
        }
    }

    /// Enter visual selection mode.
    pub fn enter_visual_mode(&mut self) {
        self.selection_mode = SelectionMode::Visual {
            anchor_line: self.cursor_line,
            side: DiffSide::Right, // Default to right side
        };
    }

    /// Exit visual selection mode.
    pub fn exit_visual_mode(&mut self) {
        self.selection_mode = SelectionMode::Normal;
    }

    /// Get the selected line range in visual mode.
    pub fn visual_selection(&self) -> Option<(usize, usize)> {
        if let SelectionMode::Visual { anchor_line, .. } = self.selection_mode {
            let start = anchor_line.min(self.cursor_line);
            let end = anchor_line.max(self.cursor_line);
            Some((start, end))
        } else {
            None
        }
    }

    /// Check if we're in visual selection mode.
    pub fn is_visual_mode(&self) -> bool {
        matches!(self.selection_mode, SelectionMode::Visual { .. })
    }

    /// Adjust scroll to keep cursor visible.
    pub fn ensure_cursor_visible(&mut self, visible_height: usize) {
        // If cursor is above visible area
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        }
        // If cursor is below visible area
        else if self.cursor_line >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor_line.saturating_sub(visible_height) + 1;
        }
    }

    /// Scroll down by half page.
    pub fn scroll_half_down(&mut self, visible_height: usize, max_lines: usize) {
        let half = visible_height / 2;
        self.cursor_line = (self.cursor_line + half).min(max_lines.saturating_sub(1));
        self.ensure_cursor_visible(visible_height);
    }

    /// Scroll up by half page.
    pub fn scroll_half_up(&mut self, visible_height: usize) {
        let half = visible_height / 2;
        self.cursor_line = self.cursor_line.saturating_sub(half);
        self.ensure_cursor_visible(visible_height);
    }

    /// Scroll down by full page.
    pub fn scroll_page_down(&mut self, visible_height: usize, max_lines: usize) {
        self.cursor_line = (self.cursor_line + visible_height).min(max_lines.saturating_sub(1));
        self.ensure_cursor_visible(visible_height);
    }

    /// Scroll up by full page.
    pub fn scroll_page_up(&mut self, visible_height: usize) {
        self.cursor_line = self.cursor_line.saturating_sub(visible_height);
        self.ensure_cursor_visible(visible_height);
    }
}

/// Selection mode for line selection.
#[derive(Debug, Clone, Default)]
pub enum SelectionMode {
    /// Normal navigation mode.
    #[default]
    Normal,
    /// Visual selection mode (vim-style).
    Visual {
        /// The line where selection started.
        anchor_line: usize,
        /// Which side of the diff the selection is on.
        side: DiffSide,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_movement() {
        let mut nav = NavigationState::new();

        nav.cursor_down(10);
        assert_eq!(nav.cursor_line, 1);

        nav.cursor_up();
        assert_eq!(nav.cursor_line, 0);

        // Can't go below 0
        nav.cursor_up();
        assert_eq!(nav.cursor_line, 0);

        // Can't go past max
        nav.cursor_line = 9;
        nav.cursor_down(10);
        assert_eq!(nav.cursor_line, 9);
    }

    #[test]
    fn test_file_navigation() {
        let mut nav = NavigationState::new();

        nav.next_file(5);
        assert_eq!(nav.selected_file, 1);
        assert_eq!(nav.cursor_line, 0); // Reset on file change

        nav.prev_file();
        assert_eq!(nav.selected_file, 0);

        // Can't go below 0
        nav.prev_file();
        assert_eq!(nav.selected_file, 0);
    }

    #[test]
    fn test_visual_mode() {
        let mut nav = NavigationState::new();
        nav.cursor_line = 5;

        nav.enter_visual_mode();
        assert!(nav.is_visual_mode());

        nav.cursor_down(20);
        nav.cursor_down(20);
        let selection = nav.visual_selection().unwrap();
        assert_eq!(selection, (5, 7));

        nav.exit_visual_mode();
        assert!(!nav.is_visual_mode());
        assert!(nav.visual_selection().is_none());
    }

    #[test]
    fn test_scroll_visibility() {
        let mut nav = NavigationState::new();
        nav.cursor_line = 50;
        nav.scroll_offset = 0;

        nav.ensure_cursor_visible(20);
        assert_eq!(nav.scroll_offset, 31); // 50 - 20 + 1
    }
}
