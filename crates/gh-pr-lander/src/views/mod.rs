use crate::actions::{Action, NavigationAction, TextInputAction};
use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use ratatui::{layout::Rect, Frame};

// New view modules (concrete view types)
pub mod add_repository_view;
pub mod build_log_view;
pub mod command_palette_view;
pub mod confirmation_popup_view;
pub mod debug_console_view;
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
pub use key_bindings_view::KeyBindingsView;
pub use pull_request_view::MainView;
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
}

/// View trait - defines the interface that all views must implement
///
/// This allows the application to interact with views polymorphically through
/// trait objects (Box<dyn View>).
///
/// IMPORTANT: This trait must be object-safe to be used as a trait object.
/// That means:
/// - No generic methods
/// - No Self: Sized bounds
/// - All methods must use &self (not consume self)
/// - Must be Send for thread safety (actions are sent between threads)
pub trait View: std::fmt::Debug + Send {
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
}

/// Implement Clone for Box<dyn View>
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
