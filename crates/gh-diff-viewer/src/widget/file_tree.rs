//! File tree widget for navigation.

use crate::model::{FileStatus, FlatFileEntry};
use crate::traits::ThemeProvider;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Widget};

/// Widget for rendering the file tree navigation pane.
pub struct FileTreeWidget<'a, T: ThemeProvider> {
    /// Pre-flattened entries to render (from cache).
    entries: &'a [FlatFileEntry],
    /// Currently selected index in the flattened tree.
    selected: usize,
    /// Whether this pane is focused.
    focused: bool,
    /// Theme provider.
    theme: &'a T,
}

impl<'a, T: ThemeProvider> FileTreeWidget<'a, T> {
    /// Create a new file tree widget with pre-flattened entries.
    pub fn new(entries: &'a [FlatFileEntry], selected: usize, focused: bool, theme: &'a T) -> Self {
        Self {
            entries,
            selected,
            focused,
            theme,
        }
    }
}

impl<T: ThemeProvider> Widget for FileTreeWidget<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Draw border - use bright white when focused (same as diff panel)
        let border_style = if self.focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Files ");

        let inner = block.inner(area);
        block.render(area, buf);

        // Use pre-flattened entries
        let visible_height = inner.height as usize;

        // Calculate scroll offset to keep selected visible
        let scroll_offset = if self.selected >= visible_height {
            self.selected - visible_height + 1
        } else {
            0
        };

        // Render visible entries
        for (i, entry) in self
            .entries
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = i + scroll_offset == self.selected;
            self.render_entry(entry, inner.x, y, inner.width, is_selected, buf);
        }
    }
}

impl<T: ThemeProvider> FileTreeWidget<'_, T> {
    fn render_entry(
        &self,
        entry: &FlatFileEntry,
        x: u16,
        y: u16,
        width: u16,
        selected: bool,
        buf: &mut Buffer,
    ) {
        // Build the line content with tree guide lines
        let tree_prefix = entry.tree_prefix();
        let icon = entry.icon();

        // Status indicator
        let status_char = match entry.status {
            Some(FileStatus::Added) => "+",
            Some(FileStatus::Deleted) => "-",
            Some(FileStatus::Modified) => "~",
            Some(FileStatus::Renamed) => "→",
            Some(FileStatus::Copied) => "©",
            None => "",
        };

        // Stats
        let stats = if entry.additions > 0 || entry.deletions > 0 {
            format!(" +{}/-{}", entry.additions, entry.deletions)
        } else {
            String::new()
        };

        // Calculate available width for name
        // tree_prefix uses 3 chars per level ("├─ ", "│  ", etc.)
        let prefix_len = tree_prefix.chars().count() + icon.chars().count() + status_char.len();
        let stats_len = stats.len();
        let available = (width as usize).saturating_sub(prefix_len + stats_len + 1);

        // Truncate name if needed
        let name = if entry.name.len() > available {
            format!("{}…", &entry.name[..available.saturating_sub(1)])
        } else {
            entry.name.clone()
        };

        // Determine style
        let base_style = if selected {
            Style::default()
                .fg(self.theme.file_tree_selected_foreground())
                .bg(self.theme.file_tree_selected_background())
        } else {
            Style::default()
        };

        // Fill the line with background
        if selected {
            for i in 0..width {
                buf.set_string(x + i, y, " ", base_style);
            }
        }

        let mut current_x = x;

        // Render tree prefix (guide lines) - use muted color for non-selected
        let tree_style = if selected {
            base_style
        } else {
            base_style.fg(self.theme.file_tree_border())
        };
        buf.set_string(current_x, y, &tree_prefix, tree_style);
        current_x += tree_prefix.chars().count() as u16;

        // Render icon
        let icon_style = if entry.is_dir && !selected {
            base_style.fg(self.theme.file_tree_directory_foreground())
        } else {
            base_style
        };
        buf.set_string(current_x, y, icon, icon_style);
        current_x += icon.chars().count() as u16;

        // Render status
        if !status_char.is_empty() {
            let status_color = if selected {
                self.theme.file_tree_selected_foreground()
            } else {
                entry.status.map(|s| s.color()).unwrap_or(Color::White)
            };
            buf.set_string(current_x, y, status_char, base_style.fg(status_color));
            current_x += status_char.len() as u16;
        }

        // Render name
        let name_style = if entry.is_dir && !selected {
            base_style.fg(self.theme.file_tree_directory_foreground())
        } else {
            base_style
        };
        buf.set_string(current_x, y, &name, name_style);
        current_x += name.len() as u16;

        // Render stats at the end
        if !stats.is_empty() {
            let stats_x = x + width - stats.len() as u16;
            if stats_x > current_x {
                let (add_style, del_style) = if selected {
                    (base_style, base_style)
                } else {
                    (base_style.fg(Color::Green), base_style.fg(Color::Red))
                };

                // Parse and render colored stats
                let parts: Vec<&str> = stats.split('/').collect();
                if parts.len() == 2 {
                    buf.set_string(stats_x, y, parts[0], add_style);
                    buf.set_string(stats_x + parts[0].len() as u16, y, " / ", base_style);
                    buf.set_string(stats_x + parts[0].len() as u16 + 1, y, parts[1], del_style);
                } else {
                    buf.set_string(stats_x, y, &stats, base_style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FileDiff, FileTreeNode};
    use crate::traits::DefaultTheme;

    #[test]
    fn test_file_tree_widget_creation() {
        let files = vec![
            {
                let mut f = FileDiff::new("src/main.rs");
                f.additions = 10;
                f
            },
            {
                let mut f = FileDiff::new("src/lib.rs");
                f.additions = 5;
                f
            },
        ];

        let tree = FileTreeNode::from_files(&files);
        let entries = tree.flatten();
        let theme = DefaultTheme;
        let _widget = FileTreeWidget::new(&entries, 0, true, &theme);
    }
}
