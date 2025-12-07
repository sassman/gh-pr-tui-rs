//! Trait for providing theme configuration to the diff viewer.

use ratatui::style::Color;

/// Provides theme colors and styles for the diff viewer.
///
/// Implement this trait to integrate the diff viewer with your application's
/// theme system. The theme instance should be injected from the orchestrator.
///
/// # Example
///
/// ```ignore
/// use gh_diff_viewer::ThemeProvider;
/// use ratatui::style::Color;
///
/// struct MyAppTheme {
///     // ... your theme fields
/// }
///
/// impl ThemeProvider for MyAppTheme {
///     fn addition_background(&self) -> Color {
///         Color::Rgb(30, 60, 30)
///     }
///
///     fn deletion_background(&self) -> Color {
///         Color::Rgb(60, 30, 30)
///     }
///
///     // ... other methods
/// }
/// ```
pub trait ThemeProvider: Send + Sync {
    /// Background color for addition lines.
    fn addition_background(&self) -> Color;

    /// Background color for deletion lines.
    fn deletion_background(&self) -> Color;

    /// Background color for context lines.
    fn context_background(&self) -> Color {
        Color::Reset
    }

    /// Background color for hunk header lines.
    fn hunk_header_background(&self) -> Color {
        Color::Rgb(40, 40, 60)
    }

    /// Foreground color for hunk header text.
    fn hunk_header_foreground(&self) -> Color {
        Color::Cyan
    }

    /// Foreground color for line numbers.
    fn line_number_foreground(&self) -> Color {
        Color::DarkGray
    }

    /// Background color for the cursor/selected line.
    fn cursor_background(&self) -> Color {
        Color::Rgb(50, 50, 80)
    }

    /// Foreground color for the cursor/selected line.
    fn cursor_foreground(&self) -> Color {
        Color::White
    }

    /// Foreground color for comment indicators.
    fn comment_indicator_foreground(&self) -> Color {
        Color::Yellow
    }

    /// Foreground color for expansion markers.
    fn expansion_marker_foreground(&self) -> Color {
        Color::Blue
    }

    /// Background color for expansion markers.
    fn expansion_marker_background(&self) -> Color {
        Color::Rgb(40, 40, 40)
    }

    /// Border color for the file tree pane.
    fn file_tree_border(&self) -> Color {
        Color::DarkGray
    }

    /// Foreground color for selected file in file tree.
    fn file_tree_selected_foreground(&self) -> Color {
        Color::White
    }

    /// Background color for selected file in file tree.
    fn file_tree_selected_background(&self) -> Color {
        Color::Rgb(50, 50, 80)
    }

    /// Foreground color for directory names in file tree.
    fn file_tree_directory_foreground(&self) -> Color {
        Color::Blue
    }

    /// Foreground color for key hints (the key part like "c", "R").
    fn hint_key_foreground(&self) -> Color {
        Color::Yellow
    }

    /// Foreground color for hint descriptions.
    fn hint_text_foreground(&self) -> Color {
        Color::DarkGray
    }
}

/// Default theme with sensible dark-mode colors.
#[derive(Debug, Clone, Default)]
pub struct DefaultTheme;

impl ThemeProvider for DefaultTheme {
    fn addition_background(&self) -> Color {
        Color::Rgb(30, 60, 30) // dark green
    }

    fn deletion_background(&self) -> Color {
        Color::Rgb(60, 30, 30) // dark red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme() {
        let theme = DefaultTheme;
        assert_eq!(theme.addition_background(), Color::Rgb(30, 60, 30));
        assert_eq!(theme.deletion_background(), Color::Rgb(60, 30, 30));
        assert_eq!(theme.context_background(), Color::Reset);
    }
}
