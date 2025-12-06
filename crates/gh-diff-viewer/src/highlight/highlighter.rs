//! Syntax highlighter using syntect.

use crate::model::HighlightedSpan;
use ratatui::style::Color;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use syntect::highlighting::{FontStyle, Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

/// Handles syntax highlighting for diff content.
pub struct DiffHighlighter {
    /// Syntax definitions.
    syntax_set: SyntaxSet,
    /// Current theme.
    theme: Theme,
    /// Cache of highlighted content by (path, line_content) hash.
    cache: HashMap<u64, Vec<HighlightedSpan>>,
    /// Maximum cache size.
    max_cache_size: usize,
    /// Cache of syntax references by file extension (avoids repeated file lookups).
    syntax_cache: HashMap<String, usize>,
}

impl std::fmt::Debug for DiffHighlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffHighlighter")
            .field("cache_size", &self.cache.len())
            .field("max_cache_size", &self.max_cache_size)
            .finish()
    }
}

impl Default for DiffHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffHighlighter {
    /// Create a new highlighter with default settings.
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();

        Self {
            syntax_set,
            theme,
            cache: HashMap::new(),
            max_cache_size: 5000, // Increased for better performance
            syntax_cache: HashMap::new(),
        }
    }

    /// Create a highlighter with a specific theme name.
    pub fn with_theme_name(theme_name: &str) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .unwrap_or_else(|| theme_set.themes["base16-ocean.dark"].clone());

        Self {
            syntax_set,
            theme,
            cache: HashMap::new(),
            max_cache_size: 5000,
            syntax_cache: HashMap::new(),
        }
    }

    /// Create a highlighter with a custom theme.
    pub fn with_theme(theme: Theme) -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme,
            cache: HashMap::new(),
            max_cache_size: 5000,
            syntax_cache: HashMap::new(),
        }
    }

    /// Set the maximum cache size.
    pub fn with_max_cache(mut self, size: usize) -> Self {
        self.max_cache_size = size;
        self
    }

    /// Get available theme names.
    pub fn available_themes() -> Vec<&'static str> {
        vec![
            "base16-ocean.dark",
            "base16-ocean.light",
            "base16-eighties.dark",
            "base16-mocha.dark",
            "InspiredGitHub",
            "Solarized (dark)",
            "Solarized (light)",
        ]
    }

    /// Highlight a single line, returning styled spans.
    ///
    /// Results are cached for performance.
    pub fn highlight_line(&mut self, path: &str, content: &str) -> Vec<HighlightedSpan> {
        // Check highlight cache first
        let key = self.cache_key(path, content);
        if let Some(spans) = self.cache.get(&key) {
            return spans.clone();
        }

        // Get syntax from extension cache (avoid expensive find_syntax_for_file on every line)
        let syntax_idx = self.get_syntax_index(path);
        let syntax = self
            .syntax_set
            .syntaxes()
            .get(syntax_idx)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Highlight the line
        let spans = self.highlight_with_syntax(syntax, content);

        // Cache the result (with LRU-style eviction if cache is full)
        if self.cache.len() >= self.max_cache_size {
            // Remove oldest 20% instead of 50% to reduce thrashing
            let to_remove = self.max_cache_size / 5;
            let keys_to_remove: Vec<_> = self.cache.keys().take(to_remove).copied().collect();
            for key in keys_to_remove {
                self.cache.remove(&key);
            }
        }
        self.cache.insert(key, spans.clone());

        spans
    }

    /// Get syntax index for a file path (cached by extension).
    fn get_syntax_index(&mut self, path: &str) -> usize {
        // Extract extension
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check syntax cache
        if let Some(&idx) = self.syntax_cache.get(&ext) {
            return idx;
        }

        // Look up syntax (expensive - only do once per extension)
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(&ext)
            .or_else(|| {
                // Try finding by full path as fallback
                self.syntax_set.find_syntax_for_file(path).ok().flatten()
            })
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Find index in syntax set
        let idx = self
            .syntax_set
            .syntaxes()
            .iter()
            .position(|s| s.name == syntax.name)
            .unwrap_or(0);

        // Cache by extension
        self.syntax_cache.insert(ext, idx);
        idx
    }

    /// Highlight content with a specific syntax.
    fn highlight_with_syntax(
        &self,
        syntax: &syntect::parsing::SyntaxReference,
        content: &str,
    ) -> Vec<HighlightedSpan> {
        use syntect::easy::HighlightLines;

        let mut highlighter = HighlightLines::new(syntax, &self.theme);

        match highlighter.highlight_line(content, &self.syntax_set) {
            Ok(ranges) => ranges
                .iter()
                .map(|(style, text)| syntect_to_span(*style, text))
                .collect(),
            Err(_) => {
                // Fallback to plain text on error
                vec![HighlightedSpan::plain(content)]
            }
        }
    }

    /// Pre-highlight a batch of lines (call during idle time).
    pub fn prehighlight_batch(&mut self, path: &str, lines: &[&str]) {
        for line in lines {
            let _ = self.highlight_line(path, line);
        }
    }

    /// Clear the highlight cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get current cache size.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Compute a cache key for a path and content.
    fn cache_key(&self, path: &str, content: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        content.hash(&mut hasher);
        hasher.finish()
    }
}

/// Convert syntect Style to our HighlightedSpan.
fn syntect_to_span(style: Style, text: &str) -> HighlightedSpan {
    HighlightedSpan {
        text: text.to_string(),
        fg: Some(Color::Rgb(
            style.foreground.r,
            style.foreground.g,
            style.foreground.b,
        )),
        bg: if style.background.a > 0 {
            Some(Color::Rgb(
                style.background.r,
                style.background.g,
                style.background.b,
            ))
        } else {
            None
        },
        bold: style.font_style.contains(FontStyle::BOLD),
        italic: style.font_style.contains(FontStyle::ITALIC),
        underline: style.font_style.contains(FontStyle::UNDERLINE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust_code() {
        let mut highlighter = DiffHighlighter::new();
        let spans = highlighter.highlight_line("test.rs", "fn main() {}");

        // Should have multiple spans for keywords, etc.
        assert!(!spans.is_empty());

        // At least one span should have text
        assert!(spans.iter().any(|s| !s.text.is_empty()));
    }

    #[test]
    fn test_cache() {
        let mut highlighter = DiffHighlighter::new();

        // First call populates cache
        let _ = highlighter.highlight_line("test.rs", "let x = 1;");
        assert_eq!(highlighter.cache_size(), 1);

        // Second call uses cache
        let _ = highlighter.highlight_line("test.rs", "let x = 1;");
        assert_eq!(highlighter.cache_size(), 1);

        // Different content adds to cache
        let _ = highlighter.highlight_line("test.rs", "let y = 2;");
        assert_eq!(highlighter.cache_size(), 2);
    }

    #[test]
    fn test_cache_eviction() {
        let mut highlighter = DiffHighlighter::new().with_max_cache(10);

        // Fill the cache
        for i in 0..15 {
            highlighter.highlight_line("test.rs", &format!("line {}", i));
        }

        // Cache should have been evicted
        assert!(highlighter.cache_size() < 15);
    }

    #[test]
    fn test_plain_text_fallback() {
        let mut highlighter = DiffHighlighter::new();
        let spans = highlighter.highlight_line("unknown.xyz", "some content");

        // Should still produce spans
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_available_themes() {
        let themes = DiffHighlighter::available_themes();
        assert!(themes.contains(&"base16-ocean.dark"));
    }
}
