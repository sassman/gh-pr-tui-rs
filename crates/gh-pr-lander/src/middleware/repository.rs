use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use std::thread;
use std::time::Duration;

/// Repository middleware - handles data loading and async operations
pub struct RepositoryMiddleware {
    bootstrap_started: bool,
}

impl RepositoryMiddleware {
    pub fn new() -> Self {
        Self {
            bootstrap_started: false,
        }
    }
}

impl Middleware for RepositoryMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::BootstrapStart if !self.bootstrap_started => {
                self.bootstrap_started = true;

                // Clone dispatcher for the background task
                let dispatcher_clone = dispatcher.clone();

                // Spawn a background thread to simulate loading
                thread::spawn(move || {
                    log::info!("Bootstrap: Starting data loading...");

                    // Simulate loading time (3 seconds = 6 ticks)
                    thread::sleep(Duration::from_millis(6000));

                    log::info!("Bootstrap: Data loading complete");

                    // Dispatch BootstrapEnd when done
                    dispatcher_clone.dispatch(Action::BootstrapEnd);
                });

                // Pass through the BootstrapStart action
                true
            }
            _ => {
                // All other actions pass through
                true
            }
        }
    }
}
