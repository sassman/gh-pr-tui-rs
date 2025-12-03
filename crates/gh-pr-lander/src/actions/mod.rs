//! Actions module
//!
//! This module defines all actions in the application using a tagged action architecture.
//! Actions are organized by:
//! - Generic actions (Navigation, TextInput) that views translate to screen-specific actions
//! - Global actions that affect the entire application
//! - Screen-specific actions that are already targeted to a particular screen

// Shared action types
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
pub mod key_bindings;
pub mod merge_bot;
pub mod pull_request;
pub mod splash;
pub mod status_bar;

// Re-export all action types for convenience
pub use add_repository::AddRepositoryAction;
pub use bootstrap::BootstrapAction;
pub use build_log::BuildLogAction;
pub use command_palette::CommandPaletteAction;
pub use confirmation_popup::ConfirmationPopupAction;
pub use debug_console::DebugConsoleAction;
pub use global::GlobalAction;
pub use key_bindings::KeyBindingsAction;
pub use merge_bot::MergeBotAction;
pub use navigation::NavigationAction;
pub use pull_request::PullRequestAction;
pub use splash::SplashAction;
pub use status_bar::StatusBarAction;
pub use text_input::TextInputAction;

/// Root action enum - tagged by screen/domain
///
/// Actions are categorized as:
/// - `Navigate` / `TextInput`: Generic actions that need translation by the active view
/// - `Global`: Application-wide actions (quit, view management, tick)
/// - Screen-specific variants: Already targeted to a specific screen's reducer
#[derive(Debug, Clone)]
pub enum Action {
    // Generic actions (need translation by active view)
    /// Generic navigation action - will be translated by active view
    Navigate(NavigationAction),
    /// Generic text input action - will be translated by active view
    TextInput(TextInputAction),

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

    /// No-op action
    None,
}
