/// Identifies which view is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Main,
    DebugConsole,
}

/// Debug console state
#[derive(Debug, Clone)]
pub struct DebugConsoleState {
    pub visible: bool,
    pub logs: Vec<String>,
}

impl Default for DebugConsoleState {
    fn default() -> Self {
        Self {
            visible: false,
            logs: Vec::new(),
        }
    }
}

/// Application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub running: bool,
    pub active_view: ActiveView,
    pub debug_console: DebugConsoleState,
    pub theme: crate::theme::Theme,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            active_view: ActiveView::Main,
            debug_console: DebugConsoleState::default(),
            theme: crate::theme::Theme::default(),
        }
    }
}
