//! Actions module
//!
//! This module defines all actions in the application using a tagged action architecture.
//! Actions are organized by:
//! - Generic actions (Navigation, TextInput, ViewContext) that views translate to screen-specific actions
//! - Global actions that affect the entire application
//! - Screen-specific actions that are already targeted to a particular screen

// Shared action types
pub mod available_action;
pub mod context_action;
pub mod event;
pub mod global;
pub mod navigation;
pub mod text_input;

// Screen-specific action types
pub mod add_repository;
pub mod bootstrap;
pub mod build_log;
pub mod command_palette;
pub mod confirmation_popup;
pub mod debug_console;
pub mod diff_viewer;
pub mod key_bindings;
pub mod merge_bot;
pub mod pull_request;
pub mod repository;
pub mod splash;
pub mod status_bar;

// Re-export all action types for convenience
pub use add_repository::AddRepositoryAction;
pub use available_action::AvailableAction;
pub use bootstrap::BootstrapAction;
pub use build_log::BuildLogAction;
pub use command_palette::CommandPaletteAction;
pub use confirmation_popup::ConfirmationPopupAction;
pub use context_action::ContextAction;
pub use debug_console::DebugConsoleAction;
pub use diff_viewer::DiffViewerAction;
pub use event::Event;
pub use global::GlobalAction;
pub use key_bindings::KeyBindingsAction;
pub use merge_bot::MergeBotAction;
pub use navigation::NavigationAction;
pub use pull_request::PullRequestAction;
pub use repository::RepositoryAction;
pub use splash::SplashAction;
pub use status_bar::StatusBarAction;
pub use text_input::TextInputAction;

/// Root action enum - tagged by screen/domain
///
/// Actions are categorized as:
/// - `Navigate` / `TextInput` / `ViewContext`: Generic actions that need translation by the active view
/// - `Global`: Application-wide actions (quit, view management, tick)
/// - Screen-specific variants: Already targeted to a specific screen's reducer
#[derive(Debug, Clone)]
pub enum Action {
    // Events (re-enter middleware chain)
    /// Events are facts/observations that re-enter the middleware chain.
    /// Use `Action::event(Event::X)` to create - ensures visibility at call site.
    Event(Event),

    // Generic actions (need translation by active view)
    /// Generic navigation action - will be translated by active view
    Navigate(NavigationAction),
    /// Generic text input action - will be translated by active view
    TextInput(TextInputAction),
    /// Context-sensitive action - will be translated by active view
    /// These are semantic actions (Confirm, ToggleSelect, etc.) that mean
    /// different things in different views.
    ViewContext(ContextAction),

    // Global actions (no translation needed)
    /// Global application actions
    Global(GlobalAction),

    // Screen-specific actions (already targeted)
    /// Pull Request screen actions
    PullRequest(PullRequestAction),
    /// Command Palette screen actions
    CommandPalette(CommandPaletteAction),
    /// Add Repository form actions
    AddRepository(AddRepositoryAction),
    /// Key Bindings panel actions
    KeyBindings(KeyBindingsAction),
    /// Debug Console actions
    DebugConsole(DebugConsoleAction),
    /// Splash screen actions
    Splash(SplashAction),
    /// Bootstrap/initialization actions
    Bootstrap(BootstrapAction),
    /// Merge Bot actions
    MergeBot(MergeBotAction),
    /// Status Bar actions
    StatusBar(StatusBarAction),
    /// Build Log panel actions
    BuildLog(BuildLogAction),
    /// Confirmation Popup actions (approve, comment, request changes, close)
    ConfirmationPopup(ConfirmationPopupAction),
    /// Diff Viewer panel actions
    DiffViewer(DiffViewerAction),
    /// Repository management actions
    Repository(RepositoryAction),

    /// No-op action
    None,
}

impl Action {
    /// Factory method for creating events.
    ///
    /// Using this factory makes event creation visually distinct at the call site,
    /// signaling that the action will re-enter the middleware chain.
    ///
    /// # Example
    ///
    /// ```rust
    /// result_tx.send(Action::event(Event::ClientReady)).ok();
    /// ```
    pub fn event(event: Event) -> Action {
        Action::Event(event)
    }
}
