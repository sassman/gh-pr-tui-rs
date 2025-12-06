//! Background worker thread that processes actions through middleware
//!
//! This module implements the background processing architecture where:
//! - Main thread handles rendering and user input only
//! - Background thread processes all middleware (API calls, file I/O, etc.)
//! - Communication happens via channels
//!
//! Actions dispatched by middleware via Dispatcher re-enter the middleware chain,
//! enabling patterns like Event::ClientReady -> LoadRecentRepositories flow.

use crate::actions::{Action, BootstrapAction, GlobalAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// Shared state that background can read (main thread writes via reducer)
pub type SharedState = Arc<RwLock<AppState>>;

/// Spawn the background worker thread
///
/// - `action_rx`: receives actions from main thread and from Dispatcher (re-entry)
/// - `action_tx`: used to create Dispatcher for middleware to dispatch actions that re-enter
/// - `result_tx`: sends actions to main thread for reducers (non-consumed actions)
/// - `state`: shared state for middleware to read
/// - `middleware`: the middleware chain
///
/// Returns a handle that can be used for graceful shutdown
pub fn spawn_background_worker(
    action_rx: Receiver<Action>,
    action_tx: Sender<Action>,
    result_tx: Sender<Action>,
    state: SharedState,
    middleware: Vec<Box<dyn Middleware + Send>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        background_loop(action_rx, action_tx, result_tx, state, middleware);
    })
}

fn background_loop(
    action_rx: Receiver<Action>,
    action_tx: Sender<Action>,
    result_tx: Sender<Action>,
    state: SharedState,
    mut middleware: Vec<Box<dyn Middleware + Send>>,
) {
    log::info!("Background worker started");

    // Create dispatcher that re-enters actions through the middleware chain
    let dispatcher = Dispatcher::new(action_tx);

    // Tick generation for splash animation
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(150);
    let mut bootstrapping = true;

    loop {
        // Use recv_timeout to allow tick generation
        match action_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(action) => {
                // Check for shutdown signal
                if matches!(action, Action::Global(GlobalAction::Quit)) {
                    log::info!("Background worker received shutdown signal");
                    if result_tx.send(action).is_err() {
                        log::error!("Failed to send quit action to main thread");
                    }
                    break;
                }

                // Track bootstrap state for tick generation
                if matches!(action, Action::Bootstrap(BootstrapAction::End)) {
                    bootstrapping = false;
                    log::debug!("Bootstrap ended, stopping tick generation");
                }

                // Get current state snapshot for middleware
                let current_state = match state.read() {
                    Ok(s) => s.clone(),
                    Err(e) => {
                        log::error!("Failed to read shared state: {}", e);
                        continue;
                    }
                };

                // Run action through middleware chain
                let mut should_forward = true;
                for mw in &mut middleware {
                    let continue_chain = mw.handle(&action, &current_state, &dispatcher);
                    if !continue_chain {
                        should_forward = false;
                        break;
                    }
                }

                // If middleware didn't consume the action, forward to reducer
                // Note: Events are NOT forwarded - they're only for middleware observation
                // (forwarding would create an infinite loop via main loop re-routing)
                if should_forward
                    && !matches!(action, Action::Event(_))
                    && result_tx.send(action).is_err()
                {
                    log::error!("Result channel disconnected, shutting down");
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // No action received, continue to tick check
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                log::info!("Action channel disconnected, shutting down");
                break;
            }
        }

        // Generate tick if bootstrapping (for splash animation)
        if bootstrapping && last_tick.elapsed() >= tick_rate {
            if result_tx.send(Action::Global(GlobalAction::Tick)).is_err() {
                log::error!("Result channel disconnected during tick");
                break;
            }
            last_tick = Instant::now();
        }
    }

    log::info!("Background worker stopped");
}
