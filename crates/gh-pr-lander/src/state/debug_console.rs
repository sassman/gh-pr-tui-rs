//! Debug Console State

use std::collections::VecDeque;

/// Maximum number of log lines to keep in the ring buffer
pub const MAX_LOG_LINES: usize = 10_000;

/// Debug console state
#[derive(Debug, Clone)]
pub struct DebugConsoleState {
    /// Ring buffer of log lines (capped at MAX_LOG_LINES)
    pub lines: VecDeque<String>,
    /// Scroll offset (0 = bottom/newest)
    pub scroll_offset: usize,
    /// Visible height for scroll bounds
    pub visible_height: usize,
}

impl Default for DebugConsoleState {
    fn default() -> Self {
        Self {
            lines: VecDeque::with_capacity(MAX_LOG_LINES),
            scroll_offset: 0,
            visible_height: 0,
        }
    }
}

impl DebugConsoleState {
    /// Append new lines to the ring buffer, trimming old ones if over capacity
    pub fn append_lines(&mut self, new_lines: Vec<String>) {
        for line in new_lines {
            self.lines.push_back(line);
            if self.lines.len() > MAX_LOG_LINES {
                self.lines.pop_front();
            }
        }
    }
}
