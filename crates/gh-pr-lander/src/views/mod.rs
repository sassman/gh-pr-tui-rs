use crate::actions::{Action, AvailableAction, ContextAction, NavigationAction, TextInputAction};
use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use ratatui::{layout::Rect, Frame};

// New view modules (concrete view types)
pub mod add_repository_view;
pub mod build_log_view;
pub mod command_palette_view;
pub mod confirmation_popup_view;
pub mod debug_console_view;
pub mod diff_viewer_view;
pub mod key_bindings_view;
pub mod pull_request_view;
pub mod repository_tabs_view;
pub mod splash_view;
pub mod status_bar;

// Re-export concrete view types for convenience
pub use add_repository_view::AddRepositoryView;
pub use build_log_view::BuildLogView;
pub use command_palette_view::CommandPaletteView;
pub use confirmation_popup_view::ConfirmationPopupView;
pub use debug_console_view::DebugConsoleView;
pub use diff_viewer_view::DiffViewerView;
pub use key_bindings_view::KeyBindingsView;
pub use pull_request_view::PullRequestView;
pub use splash_view::SplashView;

/// View identifier - allows comparing which view is active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewId {
    Splash,
    PullRequestView,
    DebugConsole,
    CommandPalette,
    AddRepository,
    KeyBindings,
    BuildLog,
    ConfirmationPopup,
    DiffViewer,
}

/// View trait - defines the interface that all views must implement
///
/// This allows the application to interact with views polymorphically through
/// trait objects (`Box<dyn View>`).
///
/// IMPORTANT: This trait must be object-safe to be used as a trait object.
/// That means:
/// - No generic methods
/// - No Self: Sized bounds
/// - All methods must use &self (not consume self)
/// - Must be Send + Sync for thread safety (state is shared between threads)
pub trait View: std::fmt::Debug + Send + Sync {
    /// Get the unique identifier for this view type
    fn view_id(&self) -> ViewId;

    /// Render this view
    fn render(&self, state: &AppState, area: Rect, f: &mut Frame);

    /// Get the capabilities of this view (for keyboard handling)
    fn capabilities(&self, state: &AppState) -> PanelCapabilities;

    /// Clone this view into a Box
    /// This is needed because Clone requires Sized, so we provide a manual clone method
    fn clone_box(&self) -> Box<dyn View>;

    /// Translate a generic navigation action to this view's specific action.
    ///
    /// Views that handle navigation should implement this to return their
    /// screen-specific action variant. The default implementation returns None,
    /// indicating the view doesn't handle navigation.
    ///
    /// # Example
    /// ```ignore
    /// fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
    ///     match nav {
    ///         NavigationAction::Next => Some(Action::PullRequest(PullRequestAction::NavigateNext)),
    ///         NavigationAction::Previous => Some(Action::PullRequest(PullRequestAction::NavigatePrevious)),
    ///         _ => None,
    ///     }
    /// }
    /// ```
    fn translate_navigation(&self, _nav: NavigationAction) -> Option<Action> {
        None // Default: view doesn't handle navigation
    }

    /// Translate a generic text input action to this view's specific action.
    ///
    /// Views that accept text input should implement this to return their
    /// screen-specific action variant. The default implementation returns None,
    /// indicating the view doesn't handle text input.
    ///
    /// # Example
    /// ```ignore
    /// fn translate_text_input(&self, input: TextInputAction) -> Option<Action> {
    ///     match input {
    ///         TextInputAction::Char(c) => Some(Action::CommandPalette(CommandPaletteAction::Char(c))),
    ///         TextInputAction::Backspace => Some(Action::CommandPalette(CommandPaletteAction::Backspace)),
    ///         _ => None,
    ///     }
    /// }
    /// ```
    fn translate_text_input(&self, _input: TextInputAction) -> Option<Action> {
        None // Default: view doesn't handle text input
    }

    /// Translate a context-sensitive action to this view's specific action.
    ///
    /// Context actions are semantic actions (Confirm, ToggleSelect, etc.) that
    /// mean different things in different views. For example:
    /// - Confirm in PR table → Open PR in browser
    /// - Confirm in Command palette → Execute selected command
    /// - Confirm in Add repository → Submit form
    ///
    /// # Example
    /// ```ignore
    /// fn translate_context_action(&self, action: ContextAction, _state: &AppState) -> Option<Action> {
    ///     match action {
    ///         ContextAction::Confirm => Some(Action::PullRequest(PullRequestAction::OpenInBrowser)),
    ///         ContextAction::ToggleSelect => Some(Action::PullRequest(PullRequestAction::ToggleSelection)),
    ///         _ => None,
    ///     }
    /// }
    /// ```
    fn translate_context_action(
        &self,
        _action: ContextAction,
        _state: &AppState,
    ) -> Option<Action> {
        None // Default: view doesn't handle context actions
    }

    /// Check if this view accepts/handles a given action.
    ///
    /// This is used by the keyboard middleware for action gating.
    /// Return false to silently ignore the action, preventing it from
    /// "leaking" to reducers when this view is active.
    ///
    /// # Example
    /// ```ignore
    /// fn accepts_action(&self, action: &Action) -> bool {
    ///     matches!(action,
    ///         Action::PullRequest(_) |
    ///         Action::ViewContext(_) |
    ///         Action::Navigate(_)
    ///     )
    /// }
    /// ```
    fn accepts_action(&self, _action: &Action) -> bool {
        true // Default: accept all actions (backward compatible)
    }

    /// Get the available actions for this view in the current state.
    ///
    /// Returns a list of actions that can be performed, used for rendering
    /// contextual help in the UI footer (suggestions panel).
    ///
    /// # Example
    /// ```ignore
    /// fn available_actions(&self, _state: &AppState) -> Vec<AvailableAction> {
    ///     vec![
    ///         AvailableAction::primary(CommandId::Confirm, "Open"),
    ///         AvailableAction::primary(CommandId::PrMerge, "Merge"),
    ///         AvailableAction::selection(CommandId::ToggleSelect, "Select"),
    ///     ]
    /// }
    /// ```
    fn available_actions(&self, _state: &AppState) -> Vec<AvailableAction> {
        vec![] // Default: no available actions to display
    }
}

/// Implement Clone for `Box<dyn View>`
impl Clone for Box<dyn View> {
    fn clone(&self) -> Box<dyn View> {
        self.clone_box()
    }
}

/// Render the entire application UI
///
/// Rendering strategy:
/// - Render all views in the stack from bottom to top
/// - Views using `Clear` widget will preserve portions of underlying views
pub fn render(state: &AppState, area: Rect, f: &mut Frame) {
    // Render each view bottom-up so views on top render last
    for view in &state.view_stack {
        view.render(state, area, f);
    }
}
