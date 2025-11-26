use crate::logger::OwnedLogRecord;
use crate::views::{SplashView, View};

/// Debug console state
#[derive(Debug, Clone)]
pub struct DebugConsoleState {
    pub visible: bool,
    pub logs: Vec<OwnedLogRecord>,
    pub scroll_offset: usize, // Current scroll position (0 = bottom/latest)
}

impl Default for DebugConsoleState {
    fn default() -> Self {
        Self {
            visible: false,
            logs: Vec::new(),
            scroll_offset: 0,
        }
    }
}

/// Splash screen state
#[derive(Debug, Clone)]
pub struct SplashState {
    pub bootstrapping: bool,
    pub animation_frame: usize, // Current frame of the snake animation (0-15)
}

impl Default for SplashState {
    fn default() -> Self {
        Self {
            bootstrapping: true,
            animation_frame: 0,
        }
    }
}

/// Application state
pub struct AppState {
    pub running: bool,
    /// Stack of views - bottom view is the base, top views are floating overlays
    /// Views are rendered bottom-up, so the last view in the stack renders on top
    pub view_stack: Vec<Box<dyn View>>,
    pub splash: SplashState,
    pub debug_console: DebugConsoleState,
    pub theme: crate::theme::Theme,
}

impl AppState {
    /// Get the top-most (active) view from the stack
    pub fn active_view(&self) -> &Box<dyn View> {
        self.view_stack.last().expect("View stack should never be empty")
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("running", &self.running)
            .field("view_stack", &format!("{} views", self.view_stack.len()))
            .field("splash", &self.splash)
            .field("debug_console", &self.debug_console)
            .field("theme", &"<theme>")
            .finish()
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            running: self.running,
            view_stack: self.view_stack.clone(),
            splash: self.splash.clone(),
            debug_console: self.debug_console.clone(),
            theme: self.theme.clone(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            view_stack: vec![Box::new(SplashView::new())],
            splash: SplashState::default(),
            debug_console: DebugConsoleState::default(),
            theme: crate::theme::Theme::default(),
        }
    }
}
