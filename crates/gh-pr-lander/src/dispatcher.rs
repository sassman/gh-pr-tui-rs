//! Dispatcher for middleware action dispatch
//!
//! When middleware needs to dispatch actions that should re-enter the middleware chain,
//! it uses the Dispatcher. Actions dispatched via Dispatcher go back through the full
//! middleware chain (via action_tx channel to background worker).
//!
//! This enables patterns like:
//! - Event::ClientReady triggers LoadRecentRepositories
//! - LoadRecentRepositories flows through middleware and is handled by RepositoryMiddleware

use crate::actions::Action;
use std::sync::mpsc::Sender;

/// Dispatcher for sending actions through the middleware chain
///
/// Actions dispatched here re-enter the middleware chain from the beginning,
/// allowing middleware to trigger other middleware handlers.
#[derive(Clone)]
pub struct Dispatcher {
    action_tx: Sender<Action>,
}

impl Dispatcher {
    /// Create a new dispatcher with the action channel
    ///
    /// The action_tx should be a clone of the channel that feeds into the background worker,
    /// so dispatched actions re-enter the middleware chain.
    pub fn new(action_tx: Sender<Action>) -> Self {
        Self { action_tx }
    }

    /// Dispatch an action to be processed through the middleware chain
    ///
    /// The action will re-enter the middleware chain from the beginning,
    /// ensuring all middleware can observe and react to it.
    pub fn dispatch(&self, action: Action) {
        if let Err(e) = self.action_tx.send(action) {
            log::error!("Dispatcher: failed to send action: {}", e);
        }
    }
}
