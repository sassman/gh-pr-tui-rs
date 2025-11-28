use crate::keybindings::{default_keymap, Keymap};
use crate::logger::OwnedLogRecord;
use crate::views::{SplashView, View};

/// Debug console state
#[derive(Debug, Clone, Default)]
pub struct DebugConsoleState {
    pub visible: bool,
    pub logs: Vec<OwnedLogRecord>,
    pub scroll_offset: usize, // Current scroll position (0 = bottom/latest)
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

/// Main view state
#[derive(Debug, Clone, Default)]
pub struct MainViewState {
    pub selected_repository: usize, // Currently selected repository index
}

/// Command palette state
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub query: String,         // Search query
    pub selected_index: usize, // Currently selected command index
}

/// Application state
pub struct AppState {
    pub running: bool,
    /// Stack of views - bottom view is the base, top views are floating overlays
    /// Views are rendered bottom-up, so the last view in the stack renders on top
    pub view_stack: Vec<Box<dyn View>>,
    pub splash: SplashState,
    pub main_view: MainViewState,
    pub debug_console: DebugConsoleState,
    pub command_palette: CommandPaletteState,
    pub theme: crate::theme::Theme,
    /// The keymap containing all keybindings
    pub keymap: Keymap,
}

impl AppState {
    /// Get the top-most (active) view from the stack
    pub fn active_view(&self) -> &dyn View {
        self.view_stack
            .last()
            .expect("View stack should never be empty")
            .as_ref()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("running", &self.running)
            .field("view_stack", &format!("{} views", self.view_stack.len()))
            .field("splash", &self.splash)
            .field("main_view", &self.main_view)
            .field("debug_console", &self.debug_console)
            .field("command_palette", &self.command_palette)
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
            main_view: self.main_view.clone(),
            debug_console: self.debug_console.clone(),
            command_palette: self.command_palette.clone(),
            theme: self.theme.clone(),
            keymap: self.keymap.clone(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            view_stack: vec![Box::new(SplashView::new())],
            splash: SplashState::default(),
            main_view: MainViewState::default(),
            debug_console: DebugConsoleState::default(),
            command_palette: CommandPaletteState::default(),
            theme: crate::theme::Theme::default(),
            keymap: default_keymap(),
        }
    }
}
