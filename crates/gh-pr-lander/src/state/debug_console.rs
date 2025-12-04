//! Debug Console State

/// Debug console state
#[derive(Debug, Clone, Default)]
pub struct DebugConsoleState {
    /// Current log lines (updated by middleware when console is visible)
    pub lines: Vec<String>,
    /// Scroll offset (0 = bottom/newest)
    pub scroll_offset: usize,
    /// Visible height for scroll bounds
    pub visible_height: usize,
}
