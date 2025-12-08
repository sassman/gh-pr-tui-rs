//! Main state for the diff viewer widget.

use super::{CommentEditor, NavigationState};
use crate::action::DiffAction;
use crate::event::{DiffEvent, ExpandDirection};
use crate::model::{
    DiffLine, DiffSide, FileDiff, FileTreeNode, FlatFileEntry, LineKind, PendingComment,
    PullRequestDiff, ReviewEvent,
};
use std::collections::HashSet;

/// Main state for the diff viewer widget.
#[derive(Debug, Clone)]
pub struct DiffViewerState {
    /// The pull request diff being viewed.
    pub diff: PullRequestDiff,
    /// File tree for navigation.
    pub file_tree: FileTreeNode,
    /// Navigation state.
    pub nav: NavigationState,
    /// Pending comments (not yet submitted).
    pub pending_comments: Vec<PendingComment>,
    /// Currently active comment editor (if any).
    pub comment_editor: Option<CommentEditor>,
    /// Whether the review submission popup is visible.
    pub show_review_popup: bool,
    /// Currently selected review event type.
    pub selected_review_event: ReviewEvent,
    /// Viewport height (for scroll calculations)
    pub viewport_height: usize,

    // === Cached state for rendering performance ===
    /// Cached flattened file tree (invalidated on expand/collapse).
    cached_flat_tree: Option<Vec<FlatFileEntry>>,
    /// Cached comment line numbers per file path (invalidated on comment add/remove).
    cached_comment_lines: Option<std::collections::HashMap<String, HashSet<u32>>>,
}

impl DiffViewerState {
    /// Create a new diff viewer state.
    pub fn new(diff: PullRequestDiff) -> Self {
        let file_tree = FileTreeNode::from_files(&diff.files);

        let mut state = Self {
            diff,
            file_tree,
            nav: NavigationState::new(),
            pending_comments: Vec::new(),
            comment_editor: None,
            show_review_popup: false,
            selected_review_event: ReviewEvent::Comment,
            viewport_height: 20, // Default, will be updated by orchestrator
            cached_flat_tree: None,
            cached_comment_lines: None,
        };

        // Sync file tree cursor to the first actual file (skip directories)
        state.sync_file_tree_cursor_to_selected_file();
        state
    }

    /// Sync the file tree cursor position to match the currently selected file.
    /// This finds the first file entry in the flattened tree that matches
    /// the selected file index and moves the cursor there.
    fn sync_file_tree_cursor_to_selected_file(&mut self) {
        // Clone the target path to avoid borrow issues
        let target_path = self
            .diff
            .files
            .get(self.nav.selected_file)
            .map(|f| f.path.clone());

        if let Some(target_path) = target_path {
            let entries = self.flat_tree();
            for (i, entry) in entries.iter().enumerate() {
                if let Some(ref path) = entry.path {
                    if *path == target_path {
                        self.nav.file_tree_cursor = i;
                        return;
                    }
                }
            }
        }
    }

    // === Cache accessors ===

    /// Get flattened file tree (cached).
    pub fn flat_tree(&mut self) -> &[FlatFileEntry] {
        if self.cached_flat_tree.is_none() {
            self.cached_flat_tree = Some(self.file_tree.flatten());
        }
        self.cached_flat_tree.as_ref().unwrap()
    }

    /// Get the length of the flattened file tree (uses cache).
    pub fn flat_tree_len(&mut self) -> usize {
        self.flat_tree().len()
    }

    /// Get comment line numbers for a file (cached).
    /// Returns a cloned HashSet to avoid borrow issues.
    pub fn comment_lines_for_file(&mut self, path: &str) -> HashSet<u32> {
        if self.cached_comment_lines.is_none() {
            let mut map: std::collections::HashMap<String, HashSet<u32>> =
                std::collections::HashMap::new();
            for comment in &self.pending_comments {
                map.entry(comment.path.clone())
                    .or_default()
                    .insert(comment.position.line);
            }
            self.cached_comment_lines = Some(map);
        }

        self.cached_comment_lines
            .as_ref()
            .unwrap()
            .get(path)
            .cloned()
            .unwrap_or_default()
    }

    // === Cache invalidation ===

    /// Invalidate the flat tree cache (call after expand/collapse).
    fn invalidate_flat_tree_cache(&mut self) {
        self.cached_flat_tree = None;
    }

    /// Invalidate the comment lines cache (call after comment add/remove).
    fn invalidate_comment_cache(&mut self) {
        self.cached_comment_lines = None;
    }

    /// Get the currently selected file diff.
    pub fn current_file(&self) -> Option<&FileDiff> {
        self.diff.files.get(self.nav.selected_file)
    }

    /// Get the currently selected file diff mutably.
    pub fn current_file_mut(&mut self) -> Option<&mut FileDiff> {
        self.diff.files.get_mut(self.nav.selected_file)
    }

    /// Get the current line at cursor.
    pub fn current_line(&self) -> Option<&DiffLine> {
        let file = self.current_file()?;
        self.get_line_at_display_index(file, self.nav.cursor_line)
    }

    /// Get line at a display index (flattened across hunks).
    fn get_line_at_display_index<'a>(
        &self,
        file: &'a FileDiff,
        display_idx: usize,
    ) -> Option<&'a DiffLine> {
        let mut current_idx = 0;
        for hunk in &file.hunks {
            // Skip hunk header line
            if current_idx == display_idx {
                return None; // Cursor on hunk header
            }
            current_idx += 1;

            for line in &hunk.lines {
                if current_idx == display_idx {
                    return Some(line);
                }
                current_idx += 1;
            }
        }
        None
    }

    /// Get total number of display lines for current file.
    pub fn current_file_line_count(&self) -> usize {
        self.current_file().map(|f| f.total_lines()).unwrap_or(0)
    }

    /// Get display line indices of all hunk headers in the current file.
    fn hunk_header_lines(&self) -> Vec<usize> {
        let Some(file) = self.current_file() else {
            return Vec::new();
        };

        let mut indices = Vec::new();
        let mut display_idx = 0;

        for hunk in &file.hunks {
            // The hunk header is at display_idx
            indices.push(display_idx);
            // Move past the header
            display_idx += 1;
            // Move past all lines in the hunk
            display_idx += hunk.lines.len();
        }

        indices
    }

    /// Jump to the next hunk header (returns true if jumped).
    fn jump_to_next_hunk(&mut self) -> bool {
        let headers = self.hunk_header_lines();
        // Find first header after current cursor
        if let Some(&next) = headers.iter().find(|&&idx| idx > self.nav.cursor_line) {
            self.nav.cursor_line = next;
            // Scroll so hunk header is near top of viewport for better visibility
            self.nav.scroll_cursor_near_top(self.viewport_height);
            true
        } else {
            false
        }
    }

    /// Jump to the previous hunk header (returns true if jumped).
    fn jump_to_prev_hunk(&mut self) -> bool {
        let headers = self.hunk_header_lines();
        // Find last header before current cursor
        if let Some(&prev) = headers
            .iter()
            .rev()
            .find(|&&idx| idx < self.nav.cursor_line)
        {
            self.nav.cursor_line = prev;
            // Scroll so hunk header is near top of viewport for better visibility
            self.nav.scroll_cursor_near_top(self.viewport_height);
            true
        } else {
            false
        }
    }

    /// Check if the comment editor is currently active.
    pub fn is_editing_comment(&self) -> bool {
        self.comment_editor.is_some()
    }

    /// Handle an action, returning any resulting events.
    ///
    /// This is the main entry point for processing user actions. The orchestrating
    /// application is responsible for mapping key events to DiffAction variants.
    pub fn handle_action(&mut self, action: DiffAction) -> Vec<DiffEvent> {
        let mut events = Vec::new();

        // Route based on current mode
        if self.comment_editor.is_some() && action.is_comment_action() {
            if let Some(event) = self.handle_comment_action(&action) {
                events.push(event);
            }
            return events;
        }

        if self.show_review_popup {
            if let Some(event) = self.handle_review_popup_action(&action) {
                events.push(event);
            }
            return events;
        }

        // Handle the action
        if let Some(event) = self.handle_normal_action(&action) {
            events.push(event);
        }

        events
    }

    /// Handle actions in normal mode (not editing comment, not in review popup).
    fn handle_normal_action(&mut self, action: &DiffAction) -> Option<DiffEvent> {
        match action {
            // === Focus Management ===
            DiffAction::ToggleFocus => {
                self.nav.toggle_focus();
                None
            }
            DiffAction::FocusFileTree => {
                self.nav.file_tree_focused = true;
                None
            }
            DiffAction::FocusDiffContent => {
                self.nav.file_tree_focused = false;
                None
            }
            DiffAction::ToggleFileTree => {
                self.nav.toggle_file_tree();
                None
            }

            // === Cursor Navigation ===
            DiffAction::CursorDown => {
                if self.nav.file_tree_focused {
                    let len = self.flat_tree_len();
                    if self.nav.file_tree_cursor + 1 < len {
                        self.nav.file_tree_cursor += 1;
                    }
                    // Auto-select file when navigating in file tree
                    self.auto_select_file_at_cursor()
                } else {
                    self.nav.cursor_down(self.current_file_line_count());
                    self.nav.ensure_cursor_visible(self.viewport_height);
                    self.emit_selection_changed()
                }
            }
            DiffAction::CursorUp => {
                if self.nav.file_tree_focused {
                    self.nav.file_tree_cursor = self.nav.file_tree_cursor.saturating_sub(1);
                    // Auto-select file when navigating in file tree
                    self.auto_select_file_at_cursor()
                } else {
                    self.nav.cursor_up();
                    self.nav.ensure_cursor_visible(self.viewport_height);
                    self.emit_selection_changed()
                }
            }
            DiffAction::CursorFirst => {
                if self.nav.file_tree_focused {
                    self.nav.file_tree_cursor = 0;
                    self.auto_select_file_at_cursor()
                } else {
                    self.nav.cursor_first();
                    // cursor_first already sets scroll_offset to 0
                    self.emit_selection_changed()
                }
            }
            DiffAction::CursorLast => {
                if self.nav.file_tree_focused {
                    let len = self.flat_tree_len();
                    self.nav.file_tree_cursor = len.saturating_sub(1);
                    self.auto_select_file_at_cursor()
                } else {
                    let max = self.current_file_line_count();
                    self.nav.cursor_last(max);
                    self.nav.ensure_cursor_visible(self.viewport_height);
                    self.emit_selection_changed()
                }
            }

            // === File Navigation ===
            DiffAction::NextFile => {
                self.nav.next_file(self.diff.files.len());
                self.emit_selection_changed()
            }
            DiffAction::PrevFile => {
                self.nav.prev_file();
                self.emit_selection_changed()
            }
            DiffAction::SelectFile(idx) => {
                self.nav.select_file(*idx, self.diff.files.len());
                self.emit_selection_changed()
            }

            // === Hunk Navigation ===
            DiffAction::NextHunk => {
                if !self.nav.file_tree_focused {
                    self.jump_to_next_hunk();
                }
                self.emit_selection_changed()
            }
            DiffAction::PrevHunk => {
                if !self.nav.file_tree_focused {
                    self.jump_to_prev_hunk();
                }
                self.emit_selection_changed()
            }

            // === Scrolling ===
            DiffAction::ScrollHalfDown => {
                self.nav
                    .scroll_half_down(self.viewport_height, self.current_file_line_count());
                None
            }
            DiffAction::ScrollHalfUp => {
                self.nav.scroll_half_up(self.viewport_height);
                None
            }
            DiffAction::ScrollPageDown => {
                self.nav
                    .scroll_page_down(self.viewport_height, self.current_file_line_count());
                None
            }
            DiffAction::ScrollPageUp => {
                self.nav.scroll_page_up(self.viewport_height);
                None
            }

            // === File Tree Operations ===
            DiffAction::ToggleTreeNode => {
                if self.nav.file_tree_focused {
                    self.select_file_at_cursor()
                } else {
                    None
                }
            }
            DiffAction::ExpandTreeNode | DiffAction::CollapseTreeNode => {
                // Simplified - toggle handles both
                if self.nav.file_tree_focused {
                    self.toggle_directory_at_cursor();
                }
                None
            }

            // === Visual Mode ===
            DiffAction::EnterVisualMode => {
                if !self.nav.is_visual_mode() {
                    self.nav.enter_visual_mode();
                }
                None
            }
            DiffAction::ExitVisualMode => {
                self.nav.exit_visual_mode();
                None
            }

            // === Comments ===
            DiffAction::StartComment => self.open_comment_editor(),
            DiffAction::CommitComment => self.submit_comment(),
            DiffAction::CancelComment => {
                self.comment_editor = None;
                None
            }

            // Comment editing actions (handled here when not in comment mode)
            DiffAction::CommentInsertChar(_)
            | DiffAction::CommentBackspace
            | DiffAction::CommentDelete
            | DiffAction::CommentCursorLeft
            | DiffAction::CommentCursorRight
            | DiffAction::CommentCursorHome
            | DiffAction::CommentCursorEnd
            | DiffAction::CommentNewline => None, // Only valid when editing

            // === Context Expansion ===
            DiffAction::ExpandContextAbove => self.request_expand(ExpandDirection::Up),
            DiffAction::ExpandContextBelow => self.request_expand(ExpandDirection::Down),

            // === Review ===
            DiffAction::ShowReviewPopup => {
                self.show_review_popup = true;
                None
            }
            DiffAction::HideReviewPopup => {
                self.show_review_popup = false;
                None
            }
            DiffAction::ReviewOptionNext
            | DiffAction::ReviewOptionPrev
            | DiffAction::SubmitReview => {
                // These are only valid in review popup mode
                None
            }

            // === General ===
            DiffAction::Close => Some(DiffEvent::Close),

            // === Viewport ===
            DiffAction::SetViewport { width: _, height } => {
                self.viewport_height = *height as usize;
                None
            }
        }
    }

    /// Handle actions when the comment editor is active.
    fn handle_comment_action(&mut self, action: &DiffAction) -> Option<DiffEvent> {
        let editor = self.comment_editor.as_mut()?;

        match action {
            DiffAction::CommitComment => self.submit_comment(),
            DiffAction::CancelComment => {
                self.comment_editor = None;
                None
            }
            DiffAction::CommentInsertChar(c) => {
                editor.insert_char(*c);
                None
            }
            DiffAction::CommentBackspace => {
                editor.delete_char_before();
                None
            }
            DiffAction::CommentDelete => {
                editor.delete_char_at();
                None
            }
            DiffAction::CommentCursorLeft => {
                editor.cursor_left();
                None
            }
            DiffAction::CommentCursorRight => {
                editor.cursor_right();
                None
            }
            DiffAction::CommentCursorHome => {
                editor.cursor_home();
                None
            }
            DiffAction::CommentCursorEnd => {
                editor.cursor_end();
                None
            }
            DiffAction::CommentNewline => {
                editor.insert_newline();
                None
            }
            _ => None, // Other actions not handled in comment mode
        }
    }

    /// Handle actions when the review popup is visible.
    fn handle_review_popup_action(&mut self, action: &DiffAction) -> Option<DiffEvent> {
        match action {
            DiffAction::ReviewOptionPrev => {
                self.selected_review_event = match self.selected_review_event {
                    ReviewEvent::Approve => ReviewEvent::Comment,
                    ReviewEvent::RequestChanges => ReviewEvent::Approve,
                    ReviewEvent::Comment => ReviewEvent::RequestChanges,
                };
                None
            }
            DiffAction::ReviewOptionNext => {
                self.selected_review_event = match self.selected_review_event {
                    ReviewEvent::Approve => ReviewEvent::RequestChanges,
                    ReviewEvent::RequestChanges => ReviewEvent::Comment,
                    ReviewEvent::Comment => ReviewEvent::Approve,
                };
                None
            }
            DiffAction::SubmitReview => {
                self.show_review_popup = false;
                Some(DiffEvent::SubmitReview {
                    event: self.selected_review_event,
                    body: None,
                })
            }
            DiffAction::HideReviewPopup | DiffAction::Close => {
                self.show_review_popup = false;
                None
            }
            _ => None, // Other actions not handled in review popup
        }
    }

    /// Open the comment editor for the current line/selection.
    /// If a comment already exists at this position, edit it instead of creating a new one.
    fn open_comment_editor(&mut self) -> Option<DiffEvent> {
        let file = self.current_file()?;
        let file_path = file.path.clone();

        // Determine the side and line number
        let (side, line) = if let Some(diff_line) = self.current_line() {
            let side = match diff_line.kind {
                LineKind::Deletion => DiffSide::Left,
                _ => DiffSide::Right,
            };
            let line_no = diff_line.new_line.or(diff_line.old_line)?;
            (side, line_no)
        } else {
            return None; // Can't comment on hunk headers
        };

        // Check for visual selection
        let visual_selection = self.nav.visual_selection();

        // Check if there's an existing comment at this position
        let existing_comment = self
            .pending_comments
            .iter()
            .enumerate()
            .find(|(_, c)| c.path == file_path && c.position.line == line);

        self.comment_editor = if let Some((idx, comment)) = existing_comment {
            // Edit existing comment
            Some(CommentEditor::edit_existing(
                file_path,
                comment.position.clone(),
                &comment.body,
                idx,
                comment.github_id,
            ))
        } else if let Some((start_idx, end_idx)) = visual_selection {
            // New multi-line comment
            let start_line = line.saturating_sub((end_idx - start_idx) as u32);
            Some(CommentEditor::new_range(file_path, side, start_line, line))
        } else {
            // New single-line comment
            Some(CommentEditor::new(file_path, side, line))
        };

        // Exit visual mode if we were in it
        self.nav.exit_visual_mode();
        None
    }

    /// Submit the current comment.
    fn submit_comment(&mut self) -> Option<DiffEvent> {
        let editor = self.comment_editor.take()?;

        if editor.is_empty() {
            // If editing an existing comment and body is now empty, delete it
            if let Some(idx) = editor.editing_index {
                if idx < self.pending_comments.len() {
                    self.pending_comments.remove(idx);
                    self.invalidate_comment_cache();
                    return Some(DiffEvent::CommentDeleted(idx));
                }
            }
            return None;
        }

        if let Some(idx) = editor.editing_index {
            // Update existing comment
            if let Some(comment) = self.pending_comments.get_mut(idx) {
                comment.body = editor.body.clone();
                self.invalidate_comment_cache();
                return Some(DiffEvent::CommentEdited {
                    index: idx,
                    body: editor.body,
                });
            }
        }

        // Add new comment
        let comment = PendingComment::new(editor.file_path, editor.position, editor.body);
        self.pending_comments.push(comment.clone());
        self.invalidate_comment_cache();
        Some(DiffEvent::CommentAdded(comment))
    }

    /// Request context expansion.
    fn request_expand(&self, direction: ExpandDirection) -> Option<DiffEvent> {
        let file = self.current_file()?;
        let line = self.current_line()?;
        let from_line = line.display_line_number()?;

        let commit_sha = match direction {
            ExpandDirection::Up => &self.diff.base_sha,
            ExpandDirection::Down => &self.diff.head_sha,
        };

        Some(DiffEvent::RequestContext {
            file_path: file.path.clone(),
            commit_sha: commit_sha.clone(),
            direction,
            from_line,
            count: 10, // Default expansion count
        })
    }

    /// Select the file at the current cursor position in the file tree.
    /// This also switches focus to the diff content pane.
    fn select_file_at_cursor(&mut self) -> Option<DiffEvent> {
        // Get entry info from cache - extract file_tree_cursor first to avoid borrow issues
        let cursor_pos = self.nav.file_tree_cursor;
        let (is_dir, path, name) = {
            let entries = self.flat_tree();
            let entry = entries.get(cursor_pos)?;
            (entry.is_dir, entry.path.clone(), entry.name.clone())
        };

        if is_dir {
            // Toggle directory
            self.toggle_directory_by_name(&name);
            None
        } else if let Some(path) = path {
            // Find the file index
            let file_idx = self.diff.files.iter().position(|f| f.path == path)?;
            self.nav.select_file(file_idx, self.diff.files.len());
            self.nav.file_tree_focused = false;
            Some(DiffEvent::FileSelected {
                file_path: path,
                file_index: file_idx,
            })
        } else {
            None
        }
    }

    /// Auto-select the file at cursor without switching focus.
    /// Used during navigation to preview files while staying in file tree.
    fn auto_select_file_at_cursor(&mut self) -> Option<DiffEvent> {
        let cursor_pos = self.nav.file_tree_cursor;
        let path = {
            let entries = self.flat_tree();
            let entry = entries.get(cursor_pos)?;
            // Only select if it's a file, not a directory
            if entry.is_dir {
                return None;
            }
            entry.path.clone()
        };

        if let Some(path) = path {
            // Find the file index and select it (but don't switch focus)
            let file_idx = self.diff.files.iter().position(|f| f.path == path)?;
            self.nav.select_file(file_idx, self.diff.files.len());
            Some(DiffEvent::FileSelected {
                file_path: path,
                file_index: file_idx,
            })
        } else {
            None
        }
    }

    /// Toggle a directory by name and invalidate cache.
    fn toggle_directory_by_name(&mut self, name: &str) {
        self.file_tree.toggle_at_path(name);
        self.invalidate_flat_tree_cache();
    }

    /// Toggle a directory at the current cursor position.
    fn toggle_directory_at_cursor(&mut self) {
        let cursor_pos = self.nav.file_tree_cursor;
        let name = {
            let entries = self.flat_tree();
            entries
                .get(cursor_pos)
                .filter(|e| e.is_dir)
                .map(|e| e.name.clone())
        };
        if let Some(name) = name {
            self.toggle_directory_by_name(&name);
        }
    }

    /// Emit a selection changed event.
    fn emit_selection_changed(&self) -> Option<DiffEvent> {
        let file = self.current_file()?;
        let line_no = self.current_line().and_then(|l| l.display_line_number());

        Some(DiffEvent::SelectionChanged {
            file_path: file.path.clone(),
            line: line_no,
        })
    }

    /// Get pending comments for a specific file.
    pub fn comments_for_file(&self, path: &str) -> Vec<&PendingComment> {
        self.pending_comments
            .iter()
            .filter(|c| c.path == path)
            .collect()
    }

    /// Delete a pending comment by index.
    pub fn delete_pending_comment(&mut self, index: usize) -> Option<DiffEvent> {
        if index < self.pending_comments.len() {
            self.pending_comments.remove(index);
            Some(DiffEvent::CommentDeleted(index))
        } else {
            None
        }
    }

    /// Insert expanded context lines into a file.
    pub fn insert_expanded_lines(
        &mut self,
        file_path: &str,
        direction: ExpandDirection,
        at_line: u32,
        lines: Vec<String>,
    ) {
        let file = match self.diff.files.iter_mut().find(|f| f.path == file_path) {
            Some(f) => f,
            None => return,
        };

        let lines_count = lines.len();

        // Find the hunk that contains the target line and insert context
        for hunk in &mut file.hunks {
            let hunk_end = hunk.new_start + hunk.new_count;

            if at_line >= hunk.new_start && at_line <= hunk_end {
                let insert_pos = match direction {
                    ExpandDirection::Up => 0,
                    ExpandDirection::Down => hunk.lines.len(),
                };

                let mut new_lines: Vec<DiffLine> = lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, content)| {
                        let line_no = match direction {
                            ExpandDirection::Up => {
                                at_line.saturating_sub(lines_count as u32 - i as u32)
                            }
                            ExpandDirection::Down => at_line + i as u32 + 1,
                        };
                        let mut line = DiffLine::context(content, line_no, line_no);
                        line.is_expanded = true;
                        line
                    })
                    .collect();

                // Insert at the appropriate position
                let tail = hunk.lines.split_off(insert_pos);
                hunk.lines.append(&mut new_lines);
                hunk.lines.extend(tail);

                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Hunk;

    fn sample_diff() -> PullRequestDiff {
        let mut diff = PullRequestDiff::new("base", "head");
        let mut file = FileDiff::new("src/main.rs");
        let mut hunk = Hunk::new(1, 5, 1, 6);
        hunk.lines.push(DiffLine::context("fn main() {", 1, 1));
        hunk.lines.push(DiffLine::deletion("    old_line()", 2));
        hunk.lines.push(DiffLine::addition("    new_line()", 2));
        hunk.lines.push(DiffLine::context("}", 3, 4));
        file.hunks.push(hunk);
        file.recalculate_stats();
        diff.files.push(file);
        diff.recalculate_totals();
        diff
    }

    #[test]
    fn test_new_state() {
        let diff = sample_diff();
        let state = DiffViewerState::new(diff);

        assert_eq!(state.diff.files.len(), 1);
        assert_eq!(state.nav.selected_file, 0);
        assert!(state.pending_comments.is_empty());
    }

    #[test]
    fn test_navigation() {
        let diff = sample_diff();
        let mut state = DiffViewerState::new(diff);

        // Focus diff content pane for navigation test
        state.nav.file_tree_focused = false;

        // Move down
        let events = state.handle_action(DiffAction::CursorDown);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DiffEvent::SelectionChanged { .. }));
        assert_eq!(state.nav.cursor_line, 1);

        // Move up
        state.handle_action(DiffAction::CursorUp);
        assert_eq!(state.nav.cursor_line, 0);
    }

    #[test]
    fn test_visual_mode() {
        let diff = sample_diff();
        let mut state = DiffViewerState::new(diff);

        // Enter visual mode
        state.handle_action(DiffAction::EnterVisualMode);
        assert!(state.nav.is_visual_mode());

        // Exit with action
        state.handle_action(DiffAction::ExitVisualMode);
        assert!(!state.nav.is_visual_mode());
    }

    #[test]
    fn test_close_event() {
        let diff = sample_diff();
        let mut state = DiffViewerState::new(diff);

        let events = state.handle_action(DiffAction::Close);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DiffEvent::Close));
    }

    #[test]
    fn test_review_popup() {
        let diff = sample_diff();
        let mut state = DiffViewerState::new(diff);

        // Show popup
        state.handle_action(DiffAction::ShowReviewPopup);
        assert!(state.show_review_popup);

        // Initial state is Comment, Next goes to Approve
        state.handle_action(DiffAction::ReviewOptionNext);
        assert_eq!(state.selected_review_event, ReviewEvent::Approve);

        // Next again goes to RequestChanges
        state.handle_action(DiffAction::ReviewOptionNext);
        assert_eq!(state.selected_review_event, ReviewEvent::RequestChanges);

        // Submit
        let events = state.handle_action(DiffAction::SubmitReview);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DiffEvent::SubmitReview { .. }));
        assert!(!state.show_review_popup);
    }

    #[test]
    fn test_comment_editing() {
        let diff = sample_diff();
        let mut state = DiffViewerState::new(diff);

        // Move to a line that can be commented
        state.nav.cursor_line = 1;
        state.nav.file_tree_focused = false;

        // Start comment
        state.handle_action(DiffAction::StartComment);
        assert!(state.is_editing_comment());

        // Type some characters
        state.handle_action(DiffAction::CommentInsertChar('H'));
        state.handle_action(DiffAction::CommentInsertChar('i'));

        // Verify content
        assert_eq!(state.comment_editor.as_ref().unwrap().body, "Hi");

        // Cancel
        state.handle_action(DiffAction::CancelComment);
        assert!(!state.is_editing_comment());
    }

    #[test]
    fn test_set_viewport() {
        let diff = sample_diff();
        let mut state = DiffViewerState::new(diff);

        state.handle_action(DiffAction::SetViewport {
            width: 100,
            height: 50,
        });
        assert_eq!(state.viewport_height, 50);
    }
}
