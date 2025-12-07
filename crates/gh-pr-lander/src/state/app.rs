//! Application State

use crate::keymap::{default_keymap, Keymap};
use crate::views::{SplashView, View};

use super::{
    AddRepoFormState, BuildLogState, CommandPaletteState, ConfirmationPopupState,
    DebugConsoleState, DiffViewerState, KeyBindingsPanelState, MainViewState, MergeBotState,
    SplashState, StatusBarState,
};

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
    pub add_repo_form: AddRepoFormState,
    pub merge_bot: MergeBotState,
    pub key_bindings_panel: KeyBindingsPanelState,
    pub status_bar: StatusBarState,
    pub build_log: BuildLogState,
    pub diff_viewer: DiffViewerState,
    /// Confirmation popup state (present only when popup is shown)
    pub confirmation_popup: Option<ConfirmationPopupState>,
    pub theme: gh_pr_lander_theme::Theme,
    /// The keymap containing all keybindings
    pub keymap: Keymap,
    /// Application configuration
    pub app_config: gh_pr_config::AppConfig,
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
            .field("add_repo_form", &self.add_repo_form)
            .field("merge_bot", &self.merge_bot)
            .field("key_bindings_panel", &self.key_bindings_panel)
            .field("status_bar", &self.status_bar)
            .field("build_log", &self.build_log)
            .field("diff_viewer", &self.diff_viewer)
            .field("confirmation_popup", &self.confirmation_popup)
            .field("theme", &"<theme>")
            .field("app_config", &self.app_config)
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
            add_repo_form: self.add_repo_form.clone(),
            merge_bot: self.merge_bot.clone(),
            key_bindings_panel: self.key_bindings_panel.clone(),
            status_bar: self.status_bar.clone(),
            build_log: self.build_log.clone(),
            diff_viewer: self.diff_viewer.clone(),
            confirmation_popup: self.confirmation_popup.clone(),
            theme: self.theme.clone(),
            keymap: self.keymap.clone(),
            app_config: self.app_config.clone(),
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
            add_repo_form: AddRepoFormState::default(),
            merge_bot: MergeBotState::default(),
            key_bindings_panel: KeyBindingsPanelState::default(),
            status_bar: StatusBarState::default(),
            build_log: BuildLogState::default(),
            diff_viewer: DiffViewerState::default(),
            confirmation_popup: None,
            theme: gh_pr_lander_theme::Theme::default(),
            keymap: default_keymap(),
            app_config: gh_pr_config::AppConfig::default(),
        }
    }
}
