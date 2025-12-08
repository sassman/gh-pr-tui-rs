//! Bootstrap actions
//!
//! Actions for application initialization and configuration loading.

/// Actions for application bootstrap/initialization
///
/// Note: `ClientReady` has been moved to `Event` enum as it's a fact that middleware
/// should observe. See `src/actions/event.rs` for events that re-enter middleware.
///
/// Session-related actions have been moved to `SessionAction` to maintain
/// proper separation of concerns.
#[derive(Debug, Clone)]
pub enum BootstrapAction {
    /// Bootstrap process started
    Start,
    /// Bootstrap process completed
    End,
    /// Application configuration loaded
    ConfigLoaded(gh_pr_config::AppConfig),
    /// Request to load recent repositories from config
    LoadRecentRepositories,
    /// Recent repositories loaded
    LoadRecentRepositoriesDone,
}
