use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Bootstrap middleware - manages application startup and tick generation
pub struct BootstrapMiddleware {
    tick_thread_started: Arc<Mutex<bool>>,
}

impl BootstrapMiddleware {
    pub fn new() -> Self {
        Self {
            tick_thread_started: Arc::new(Mutex::new(false)),
        }
    }
}

impl Middleware for BootstrapMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::BootstrapStart => {
                // Start tick thread if not already started
                let mut started = self.tick_thread_started.lock().unwrap();
                if !*started {
                    *started = true;

                    let dispatcher_clone = dispatcher.clone();
                    let should_continue = self.tick_thread_started.clone();

                    // Spawn tick generation thread
                    thread::spawn(move || {
                        let tick_rate = Duration::from_millis(250);
                        let mut last_tick = Instant::now();

                        loop {
                            if !*should_continue.lock().unwrap() {
                                log::debug!("Bootstrap: Tick thread terminating");
                                break;
                            }
                            // Wait for next tick
                            let now = Instant::now();
                            let elapsed = now.duration_since(last_tick);

                            if elapsed >= tick_rate {
                                dispatcher_clone.dispatch(Action::Tick);
                                last_tick = now;
                            } else {
                                // Sleep for the remaining time
                                thread::sleep(tick_rate - elapsed);
                            }
                        }
                    });

                    log::debug!("Bootstrap: Tick thread started");
                }

                // Pass through
                true
            }
            Action::BootstrapEnd => {
                // Stop the tick thread
                let mut started = self.tick_thread_started.lock().unwrap();
                *started = false;
                log::debug!("Bootstrap: Received BootstrapEnd, stopping tick thread");

                // Pass through
                true
            }
            _ => {
                // All other actions pass through
                true
            }
        }
    }
}
