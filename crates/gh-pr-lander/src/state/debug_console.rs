//! Debug Console State

use crate::logger::OwnedLogRecord;

/// Debug console state
#[derive(Debug, Clone)]
pub struct DebugConsoleState {
    pub visible: bool,
    pub logs: Vec<OwnedLogRecord>,
    pub scroll_offset: usize,  // Current scroll position (0 = bottom/latest)
    pub visible_height: usize, // Last known visible height (updated during render)
}

impl Default for DebugConsoleState {
    fn default() -> Self {
        Self {
            visible: false,
            logs: Vec::new(),
            scroll_offset: 0,
            visible_height: 20, // Reasonable default for most terminals
        }
    }
}
