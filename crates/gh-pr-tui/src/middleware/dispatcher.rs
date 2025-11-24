//! Dispatcher allows middleware to dispatch actions back to the store

use crate::actions::Action;
use tokio::sync::mpsc;

/// Dispatcher allows middleware to dispatch new actions
///
/// Actions dispatched through the Dispatcher will be processed
/// in the next event loop iteration, preventing recursion.
#[derive(Clone, Debug)]
pub struct Dispatcher {
    tx: mpsc::UnboundedSender<Action>,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new(tx: mpsc::UnboundedSender<Action>) -> Self {
        Self { tx }
    }

    /// Dispatch an action
    ///
    /// The action will be queued and processed in the next iteration
    /// of the event loop.
    pub fn dispatch(&self, action: Action) {
        if let Err(e) = self.tx.send(action) {
            log::error!("Failed to dispatch action: {}", e);
        }
    }

    /// Dispatch an action from an async context
    ///
    /// This is useful when spawning tokio tasks that need to dispatch
    /// actions back to the store.
    pub fn dispatch_async(self, action: Action) {
        tokio::spawn(async move {
            self.dispatch(action);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        dispatcher.dispatch(Action::None);

        let received = rx.try_recv();
        assert!(received.is_ok());
    }
}
