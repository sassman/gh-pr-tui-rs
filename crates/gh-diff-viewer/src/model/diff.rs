//! Diff data structures representing a pull request's changes.

use ratatui::style::Color;

/// A complete diff for a pull request.
#[derive(Debug, Clone)]
pub struct PullRequestDiff {
    /// Base commit SHA (the target branch).
    pub base_sha: String,
    /// Head commit SHA (the PR branch).
    pub head_sha: String,
    /// All files changed in this PR.
    pub files: Vec<FileDiff>,
    /// Total additions across all files.
    pub total_additions: usize,
    /// Total deletions across all files.
    pub total_deletions: usize,
}

impl PullRequestDiff {
    /// Create a new pull request diff.
    pub fn new(base_sha: impl Into<String>, head_sha: impl Into<String>) -> Self {
        Self {
            base_sha: base_sha.into(),
            head_sha: head_sha.into(),
            files: Vec::new(),
            total_additions: 0,
            total_deletions: 0,
        }
    }

    /// Recalculate totals from files.
    pub fn recalculate_totals(&mut self) {
        self.total_additions = self.files.iter().map(|f| f.additions).sum();
        self.total_deletions = self.files.iter().map(|f| f.deletions).sum();
    }
}

/// Display info for a line: (hunk_index, optional_line_index).
/// None for line_index means hunk header.
pub type DisplayLineInfo = (usize, Option<usize>);

/// A single file's diff.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Current file path (after rename if applicable).
    pub path: String,
    /// Previous file path (if renamed).
    pub old_path: Option<String>,
    /// File status.
    pub status: FileStatus,
    /// Change hunks.
    pub hunks: Vec<Hunk>,
    /// Number of added lines.
    pub additions: usize,
    /// Number of deleted lines.
    pub deletions: usize,

    // === Cached state for rendering performance ===
    /// Cached flattened display info (hunk_idx, line_idx).
    cached_display_info: Option<Vec<DisplayLineInfo>>,
    /// Cached max line number for width calculation.
    cached_max_line_no: Option<u32>,
    /// Cached display name.
    cached_display_name: Option<String>,
}

impl FileDiff {
    /// Create a new file diff.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            old_path: None,
            status: FileStatus::Modified,
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
            cached_display_info: None,
            cached_max_line_no: None,
            cached_display_name: None,
        }
    }

    /// Get the display name for the file (handles renames) - cached.
    pub fn display_name(&mut self) -> &str {
        if self.cached_display_name.is_none() {
            let name = if let Some(ref old) = self.old_path {
                if old != &self.path {
                    format!("{} → {}", old, self.path)
                } else {
                    self.path.clone()
                }
            } else {
                self.path.clone()
            };
            self.cached_display_name = Some(name);
        }
        self.cached_display_name.as_ref().unwrap()
    }

    /// Get flattened display info for rendering (cached).
    pub fn display_info(&mut self) -> &[DisplayLineInfo] {
        if self.cached_display_info.is_none() {
            let mut result = Vec::new();
            for (hunk_idx, hunk) in self.hunks.iter().enumerate() {
                result.push((hunk_idx, None)); // Hunk header
                for line_idx in 0..hunk.lines.len() {
                    result.push((hunk_idx, Some(line_idx)));
                }
            }
            self.cached_display_info = Some(result);
        }
        self.cached_display_info.as_ref().unwrap()
    }

    /// Get max line number for width calculation (cached).
    pub fn max_line_no(&mut self) -> u32 {
        if self.cached_max_line_no.is_none() {
            let max = self
                .hunks
                .iter()
                .flat_map(|h| h.lines.iter())
                .filter_map(|l| l.new_line.or(l.old_line))
                .max()
                .unwrap_or(1);
            self.cached_max_line_no = Some(max);
        }
        self.cached_max_line_no.unwrap()
    }

    /// Get line number width for display (cached via max_line_no).
    pub fn line_no_width(&mut self) -> usize {
        self.max_line_no().to_string().len().max(4)
    }

    /// Invalidate all caches (call when hunks change).
    pub fn invalidate_caches(&mut self) {
        self.cached_display_info = None;
        self.cached_max_line_no = None;
        self.cached_display_name = None;
    }

    /// Set old_path and invalidate display name cache.
    pub fn set_old_path(&mut self, old_path: Option<String>) {
        self.old_path = old_path;
        self.cached_display_name = None;
    }

    /// Recalculate line statistics from hunks.
    pub fn recalculate_stats(&mut self) {
        self.additions = self
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Addition)
            .count();
        self.deletions = self
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Deletion)
            .count();
    }

    /// Get total number of displayable lines (for scrolling).
    pub fn total_lines(&self) -> usize {
        self.hunks.iter().map(|h| h.lines.len() + 1).sum() // +1 for hunk header
    }
}

/// File status in the diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

impl FileStatus {
    /// Get a single-character representation.
    pub fn as_char(&self) -> char {
        match self {
            FileStatus::Added => 'A',
            FileStatus::Modified => 'M',
            FileStatus::Deleted => 'D',
            FileStatus::Renamed => 'R',
            FileStatus::Copied => 'C',
        }
    }

    /// Get the status color.
    pub fn color(&self) -> Color {
        match self {
            FileStatus::Added => Color::Green,
            FileStatus::Modified => Color::Yellow,
            FileStatus::Deleted => Color::Red,
            FileStatus::Renamed => Color::Cyan,
            FileStatus::Copied => Color::Cyan,
        }
    }
}

/// A contiguous region of changes (hunk).
#[derive(Debug, Clone)]
pub struct Hunk {
    /// Header line (e.g., "@@ -10,5 +10,7 @@ fn example()").
    pub header: String,
    /// Old file starting line.
    pub old_start: u32,
    /// Number of lines in old version.
    pub old_count: u32,
    /// New file starting line.
    pub new_start: u32,
    /// Number of lines in new version.
    pub new_count: u32,
    /// Lines in this hunk.
    pub lines: Vec<DiffLine>,
}

impl Hunk {
    /// Create a new hunk with the given header info.
    pub fn new(old_start: u32, old_count: u32, new_start: u32, new_count: u32) -> Self {
        Self {
            header: format!(
                "@@ -{},{} +{},{} @@",
                old_start, old_count, new_start, new_count
            ),
            old_start,
            old_count,
            new_start,
            new_count,
            lines: Vec::new(),
        }
    }

    /// Create a hunk with a function context in header.
    pub fn with_context(
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
        context: &str,
    ) -> Self {
        Self {
            header: format!(
                "@@ -{},{} +{},{} @@ {}",
                old_start, old_count, new_start, new_count, context
            ),
            old_start,
            old_count,
            new_start,
            new_count,
            lines: Vec::new(),
        }
    }
}

/// A single line in the diff.
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Line type.
    pub kind: LineKind,
    /// Line content (without leading +/-/ ).
    pub content: String,
    /// Line number in old file (for Context and Deletion).
    pub old_line: Option<u32>,
    /// Line number in new file (for Context and Addition).
    pub new_line: Option<u32>,
    /// Whether this line was expanded (not from original diff).
    pub is_expanded: bool,
    /// Cached syntax-highlighted spans (computed lazily).
    pub highlighted: Option<Vec<HighlightedSpan>>,
}

impl DiffLine {
    /// Create a new context line.
    pub fn context(content: impl Into<String>, old_line: u32, new_line: u32) -> Self {
        Self {
            kind: LineKind::Context,
            content: content.into(),
            old_line: Some(old_line),
            new_line: Some(new_line),
            is_expanded: false,
            highlighted: None,
        }
    }

    /// Create a new addition line.
    pub fn addition(content: impl Into<String>, new_line: u32) -> Self {
        Self {
            kind: LineKind::Addition,
            content: content.into(),
            old_line: None,
            new_line: Some(new_line),
            is_expanded: false,
            highlighted: None,
        }
    }

    /// Create a new deletion line.
    pub fn deletion(content: impl Into<String>, old_line: u32) -> Self {
        Self {
            kind: LineKind::Deletion,
            content: content.into(),
            old_line: Some(old_line),
            new_line: None,
            is_expanded: false,
            highlighted: None,
        }
    }

    /// Create an expansion marker.
    pub fn expansion_marker(hidden_lines: u32) -> Self {
        Self {
            kind: LineKind::Expansion,
            content: format!("... {} hidden lines ...", hidden_lines),
            old_line: None,
            new_line: None,
            is_expanded: false,
            highlighted: None,
        }
    }

    /// Get the line number to display (prefers new_line, falls back to old_line).
    pub fn display_line_number(&self) -> Option<u32> {
        self.new_line.or(self.old_line)
    }
}

/// Line type in the diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// Unchanged line (for context).
    Context,
    /// Added line (+).
    Addition,
    /// Removed line (-).
    Deletion,
    /// @@ header line.
    HunkHeader,
    /// Expansion marker (click to load more).
    Expansion,
}

impl LineKind {
    /// Get the prefix character for this line type.
    pub fn prefix(&self) -> char {
        match self {
            LineKind::Context => ' ',
            LineKind::Addition => '+',
            LineKind::Deletion => '-',
            LineKind::HunkHeader => '@',
            LineKind::Expansion => '~',
        }
    }

    /// Get the background color for this line type.
    pub fn background_color(&self) -> Option<Color> {
        match self {
            LineKind::Addition => Some(Color::Rgb(30, 60, 30)), // dark green
            LineKind::Deletion => Some(Color::Rgb(60, 30, 30)), // dark red
            LineKind::HunkHeader => Some(Color::Rgb(40, 40, 60)), // dark blue
            LineKind::Expansion => Some(Color::Rgb(40, 40, 40)), // dark gray
            LineKind::Context => None,
        }
    }
}

/// A syntax-highlighted span of text.
#[derive(Debug, Clone)]
pub struct HighlightedSpan {
    /// The text content.
    pub text: String,
    /// Foreground color.
    pub fg: Option<Color>,
    /// Background color (usually None, line background takes precedence).
    pub bg: Option<Color>,
    /// Bold style.
    pub bold: bool,
    /// Italic style.
    pub italic: bool,
    /// Underline style.
    pub underline: bool,
}

impl HighlightedSpan {
    /// Create a plain span with no styling.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }

    /// Create a span with foreground color.
    pub fn colored(text: impl Into<String>, fg: Color) -> Self {
        Self {
            text: text.into(),
            fg: Some(fg),
            bg: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_diff_display_name() {
        let mut file = FileDiff::new("src/new.rs");
        assert_eq!(file.display_name(), "src/new.rs");

        file.set_old_path(Some("src/old.rs".to_string()));
        assert_eq!(file.display_name(), "src/old.rs → src/new.rs");

        // Same path shouldn't show arrow
        file.set_old_path(Some("src/new.rs".to_string()));
        assert_eq!(file.display_name(), "src/new.rs");
    }

    #[test]
    fn test_hunk_header_format() {
        let hunk = Hunk::new(10, 5, 10, 7);
        assert_eq!(hunk.header, "@@ -10,5 +10,7 @@");

        let hunk = Hunk::with_context(10, 5, 10, 7, "fn example()");
        assert_eq!(hunk.header, "@@ -10,5 +10,7 @@ fn example()");
    }

    #[test]
    fn test_diff_line_kinds() {
        let ctx = DiffLine::context("unchanged", 5, 5);
        assert_eq!(ctx.kind, LineKind::Context);
        assert_eq!(ctx.old_line, Some(5));
        assert_eq!(ctx.new_line, Some(5));

        let add = DiffLine::addition("new line", 10);
        assert_eq!(add.kind, LineKind::Addition);
        assert_eq!(add.old_line, None);
        assert_eq!(add.new_line, Some(10));

        let del = DiffLine::deletion("removed line", 8);
        assert_eq!(del.kind, LineKind::Deletion);
        assert_eq!(del.old_line, Some(8));
        assert_eq!(del.new_line, None);
    }
}
