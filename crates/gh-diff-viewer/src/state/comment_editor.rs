//! State for the comment editor popup.

use crate::model::{CommentPosition, DiffSide};

/// State for editing a comment.
#[derive(Debug, Clone)]
pub struct CommentEditor {
    /// Position in the diff where the comment is anchored.
    pub position: CommentPosition,
    /// File path for the comment.
    pub file_path: String,
    /// The comment body being edited.
    pub body: String,
    /// Cursor position within the body.
    pub cursor: usize,
    /// Whether we're editing an existing pending comment.
    pub editing_index: Option<usize>,
    /// GitHub comment ID if editing a posted comment (for delete support).
    pub github_id: Option<u64>,
}

impl CommentEditor {
    /// Create a new comment editor for a single line.
    pub fn new(file_path: impl Into<String>, side: DiffSide, line: u32) -> Self {
        Self {
            position: CommentPosition::single(side, line),
            file_path: file_path.into(),
            body: String::new(),
            cursor: 0,
            editing_index: None,
            github_id: None,
        }
    }

    /// Create a comment editor for a line range (multiline comment).
    pub fn new_range(file_path: impl Into<String>, side: DiffSide, start: u32, end: u32) -> Self {
        Self {
            position: CommentPosition::range(side, start, end),
            file_path: file_path.into(),
            body: String::new(),
            cursor: 0,
            editing_index: None,
            github_id: None,
        }
    }

    /// Create a comment editor for editing an existing comment.
    pub fn edit_existing(
        file_path: impl Into<String>,
        position: CommentPosition,
        body: impl Into<String>,
        index: usize,
        github_id: Option<u64>,
    ) -> Self {
        let body = body.into();
        let cursor = body.len();
        Self {
            position,
            file_path: file_path.into(),
            body,
            cursor,
            editing_index: Some(index),
            github_id,
        }
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.body.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Insert a string at the cursor position.
    pub fn insert_str(&mut self, s: &str) {
        self.body.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_char_before(&mut self) {
        if self.cursor > 0 {
            // Find the start of the previous character
            let prev_char_start = self.body[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.body.remove(prev_char_start);
            self.cursor = prev_char_start;
        }
    }

    /// Delete the character at the cursor (delete key).
    pub fn delete_char_at(&mut self) {
        if self.cursor < self.body.len() {
            self.body.remove(self.cursor);
        }
    }

    /// Move cursor left by one character.
    pub fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.body[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right by one character.
    pub fn cursor_right(&mut self) {
        if self.cursor < self.body.len() {
            self.cursor = self.body[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.body.len());
        }
    }

    /// Move cursor to the start of the line.
    pub fn cursor_home(&mut self) {
        // Find the start of the current line
        self.cursor = self.body[..self.cursor]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
    }

    /// Move cursor to the end of the line.
    pub fn cursor_end(&mut self) {
        // Find the end of the current line
        self.cursor = self.body[self.cursor..]
            .find('\n')
            .map(|i| self.cursor + i)
            .unwrap_or(self.body.len());
    }

    /// Insert a newline.
    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    /// Check if the body is empty (ignoring whitespace).
    pub fn is_empty(&self) -> bool {
        self.body.trim().is_empty()
    }

    /// Get the number of lines in the body.
    pub fn line_count(&self) -> usize {
        self.body.lines().count().max(1)
    }

    /// Get the current line number (0-indexed).
    pub fn current_line(&self) -> usize {
        self.body[..self.cursor].matches('\n').count()
    }

    /// Get the cursor column on the current line.
    pub fn current_column(&self) -> usize {
        let line_start = self.body[..self.cursor]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        self.cursor - line_start
    }

    /// Clear all content.
    pub fn clear(&mut self) {
        self.body.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_delete() {
        let mut editor = CommentEditor::new("test.rs", DiffSide::Right, 10);

        editor.insert_str("Hello");
        assert_eq!(editor.body, "Hello");
        assert_eq!(editor.cursor, 5);

        editor.insert_char('!');
        assert_eq!(editor.body, "Hello!");

        editor.delete_char_before();
        assert_eq!(editor.body, "Hello");

        editor.cursor = 0;
        editor.delete_char_at();
        assert_eq!(editor.body, "ello");
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = CommentEditor::new("test.rs", DiffSide::Right, 10);
        editor.insert_str("Hello\nWorld");

        editor.cursor_home();
        assert_eq!(editor.cursor, 6); // Start of "World"

        editor.cursor_end();
        assert_eq!(editor.cursor, 11); // End of "World"

        editor.cursor = 0;
        editor.cursor_end();
        assert_eq!(editor.cursor, 5); // End of "Hello"
    }

    #[test]
    fn test_line_info() {
        let mut editor = CommentEditor::new("test.rs", DiffSide::Right, 10);
        editor.insert_str("Line 1\nLine 2\nLine 3");

        assert_eq!(editor.line_count(), 3);
        assert_eq!(editor.current_line(), 2);

        editor.cursor = 8; // "L" in "Line 2"
        assert_eq!(editor.current_line(), 1);
        assert_eq!(editor.current_column(), 1);
    }

    #[test]
    fn test_multiline_comment() {
        let editor = CommentEditor::new_range("test.rs", DiffSide::Right, 10, 15);
        assert!(editor.position.is_multiline());
        assert_eq!(editor.position.line_range(), (10, 15));
    }
}
