//! Debug Console View Model

use crate::keybindings::Keymap;
use crate::keymap::CommandId;
use crate::state::DebugConsoleState;

/// Pre-computed footer hints for keyboard shortcuts
#[derive(Debug, Clone)]
pub struct DebugConsoleFooterHints {
    /// Combined scroll hint (e.g., "j/↓/k/↑")
    pub scroll: String,
    /// Combined top/bottom hint (e.g., "gg/G")
    pub top_bottom: String,
    /// Close hint (e.g., "`")
    pub close: String,
}

/// View model for debug console - handles presentation logic
pub struct DebugConsoleViewModel<'a> {
    state: &'a DebugConsoleState,
    /// Pre-computed footer hints
    pub footer_hints: DebugConsoleFooterHints,
}

impl<'a> DebugConsoleViewModel<'a> {
    pub fn new(state: &'a DebugConsoleState, keymap: &Keymap) -> Self {
        let footer_hints = DebugConsoleFooterHints {
            scroll: format!(
                "{}/{}",
                keymap
                    .compact_hint_for_command(CommandId::NavigateNext)
                    .unwrap_or_else(|| "j/↓".to_string()),
                keymap
                    .compact_hint_for_command(CommandId::NavigatePrevious)
                    .unwrap_or_else(|| "k/↑".to_string()),
            ),
            top_bottom: format!(
                "{}/{}",
                keymap
                    .compact_hint_for_command(CommandId::NavigateToTop)
                    .unwrap_or_else(|| "gg".to_string()),
                keymap
                    .compact_hint_for_command(CommandId::NavigateToBottom)
                    .unwrap_or_else(|| "G".to_string()),
            ),
            close: keymap
                .compact_hint_for_command(CommandId::DebugToggleConsoleView)
                .unwrap_or_else(|| "`".to_string()),
        };

        Self {
            state,
            footer_hints,
        }
    }

    /// Get the visible lines based on scroll offset and available height
    ///
    /// scroll_offset = 0 means we're at the bottom (showing newest logs)
    /// scroll_offset > 0 means we've scrolled up (showing older logs)
    pub fn visible_lines(&self, available_height: usize) -> &[String] {
        let total = self.state.lines.len();

        if total == 0 || available_height == 0 {
            return &[];
        }

        // Cap scroll_offset to valid range
        let max_scroll = total.saturating_sub(available_height);
        let effective_scroll = self.state.scroll_offset.min(max_scroll);

        // end is the index AFTER the last visible line
        let end = total.saturating_sub(effective_scroll);

        // start is the index of the first visible line
        let start = end.saturating_sub(available_height);

        &self.state.lines[start..end]
    }

    /// Get the title for the debug console with scroll indicator
    pub fn title(&self) -> String {
        if self.state.scroll_offset > 0 {
            format!(
                " Debug Console (c to clear) - ↓{} ",
                self.state.scroll_offset
            )
        } else {
            " Debug Console (c to clear) ".to_string()
        }
    }
}
