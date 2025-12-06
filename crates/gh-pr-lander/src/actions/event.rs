//! Event types
//!
//! Events represent facts/observations that have occurred and should be broadcast
//! to the middleware chain. Unlike commands (imperative actions), events allow
//! middleware to react to what happened elsewhere in the system.
//!
//! Events are automatically re-injected into the middleware chain by the main loop,
//! ensuring all middleware can observe and react to them.
//!
//! ## Usage
//!
//! ```rust
//! // Send an event that will re-enter middleware chain
//! result_tx.send(Action::event(Event::ClientReady)).ok();
//!
//! // Handle event in middleware
//! Action::Event(Event::ClientReady) => {
//!     // React to the event
//! }
//! ```
//!
//! ## Naming Convention
//!
//! Events use past tense or descriptive names indicating something has happened:
//! - `ClientReady` (not `InitializeClient`)
//! - `ConfigLoaded` (not `LoadConfig`)
//! - `BootstrapCompleted` (not `CompleteBootstrap`)

/// Events that re-enter the middleware chain
///
/// These represent facts about what has happened in the system.
/// Middleware can observe these and dispatch further actions in response.
#[derive(Debug, Clone)]
pub enum Event {
    // === Bootstrap Events ===
    /// GitHub client has been initialized and is ready for API calls
    ClientReady,

    /// Bootstrap process has completed
    BootstrapCompleted,

    /// Recent repositories have been loaded from config
    RecentRepositoriesLoaded,
}
