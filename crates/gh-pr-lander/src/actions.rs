use ratatui::crossterm::event::KeyEvent;

use crate::{logger::OwnedLogRecord, views::View};

/// Actions represent all possible state changes in the application.
/// Actions are prefixed by scope to indicate which part of the app they affect.
pub enum Action {
    /// Global actions (not tied to any specific view)
    GlobalKeyPressed(KeyEvent),
    GlobalClose,
    GlobalQuit,

    /// ## View stack management
    /// Push a new view onto the stack (for modals/popups)
    PushView(Box<dyn View>),
    /// Replace entire view stack with new view (for navigation)
    ReplaceView(Box<dyn View>),

    /// ## Local actions (dispatched to active view for handling)
    /// Key pressed in active view context
    LocalKeyPressed(char),

    /// ## Navigation actions (semantic, vim-style)
    NavigateNext, // j, down arrow
    NavigatePrevious, // k, up arrow
    NavigateLeft,     // h, left arrow
    NavigateRight,    // l, right arrow

    /// ## Tab navigation actions
    TabNext, // Switch to next tab
    TabPrevious, // Switch to previous tab

    /// ## Scroll actions
    ScrollToTop, // gg
    ScrollToBottom,     // G
    ScrollPageDown,     // Page Down
    ScrollPageUp,       // Page Up
    ScrollHalfPageDown, // Ctrl+d
    ScrollHalfPageUp,   // Ctrl+u

    /// ## Debug console actions
    DebugConsoleClear, // Clear debug console logs
    DebugConsoleLogAdded(OwnedLogRecord), // New log record added

    /// ## Bootstrap actions
    BootstrapStart,
    BootstrapEnd,

    /// ## Animation/Timer actions
    Tick, // Periodic tick for animations (500ms interval)

    ///No-op action
    None,
}

impl Clone for Action {
    fn clone(&self) -> Self {
        match self {
            Self::GlobalKeyPressed(key) => Self::GlobalKeyPressed(*key),
            Self::GlobalClose => Self::GlobalClose,
            Self::GlobalQuit => Self::GlobalQuit,
            Self::PushView(view) => Self::PushView(view.clone()),
            Self::ReplaceView(view) => Self::ReplaceView(view.clone()),
            Self::LocalKeyPressed(c) => Self::LocalKeyPressed(*c),
            Self::NavigateNext => Self::NavigateNext,
            Self::NavigatePrevious => Self::NavigatePrevious,
            Self::NavigateLeft => Self::NavigateLeft,
            Self::NavigateRight => Self::NavigateRight,
            Self::TabNext => Self::TabNext,
            Self::TabPrevious => Self::TabPrevious,
            Self::ScrollToTop => Self::ScrollToTop,
            Self::ScrollToBottom => Self::ScrollToBottom,
            Self::ScrollPageDown => Self::ScrollPageDown,
            Self::ScrollPageUp => Self::ScrollPageUp,
            Self::ScrollHalfPageDown => Self::ScrollHalfPageDown,
            Self::ScrollHalfPageUp => Self::ScrollHalfPageUp,
            Self::DebugConsoleClear => Self::DebugConsoleClear,
            Self::DebugConsoleLogAdded(record) => Self::DebugConsoleLogAdded(record.clone()),
            Self::BootstrapStart => Self::BootstrapStart,
            Self::BootstrapEnd => Self::BootstrapEnd,
            Self::Tick => Self::Tick,
            Self::None => Self::None,
        }
    }
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GlobalKeyPressed(key) => f.debug_tuple("GlobalKeyPressed").field(key).finish(),
            Self::GlobalClose => write!(f, "GlobalClose"),
            Self::GlobalQuit => write!(f, "GlobalQuit"),
            Self::PushView(view) => f.debug_tuple("PushView").field(view).finish(),
            Self::ReplaceView(view) => f.debug_tuple("ReplaceView").field(view).finish(),
            Self::LocalKeyPressed(c) => f.debug_tuple("LocalKeyPressed").field(c).finish(),
            Self::NavigateNext => write!(f, "NavigateNext"),
            Self::NavigatePrevious => write!(f, "NavigatePrevious"),
            Self::NavigateLeft => write!(f, "NavigateLeft"),
            Self::NavigateRight => write!(f, "NavigateRight"),
            Self::TabNext => write!(f, "TabNext"),
            Self::TabPrevious => write!(f, "TabPrevious"),
            Self::ScrollToTop => write!(f, "ScrollToTop"),
            Self::ScrollToBottom => write!(f, "ScrollToBottom"),
            Self::ScrollPageDown => write!(f, "ScrollPageDown"),
            Self::ScrollPageUp => write!(f, "ScrollPageUp"),
            Self::ScrollHalfPageDown => write!(f, "ScrollHalfPageDown"),
            Self::ScrollHalfPageUp => write!(f, "ScrollHalfPageUp"),
            Self::DebugConsoleClear => write!(f, "DebugConsoleClear"),
            Self::DebugConsoleLogAdded(record) => {
                f.debug_tuple("DebugConsoleLogAdded").field(record).finish()
            }
            Self::BootstrapStart => write!(f, "BootstrapStart"),
            Self::BootstrapEnd => write!(f, "BootstrapEnd"),
            Self::Tick => write!(f, "Tick"),
            Self::None => write!(f, "None"),
        }
    }
}
