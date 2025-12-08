//! Debug Console Middleware
//!
//! Manages the FileLogReader lifecycle with its own polling thread:
//! - Initialize reader and start polling thread when debug console is opened
//! - Polling thread dispatches line updates at regular intervals
//! - Stop polling thread when console is closed

use crate::actions::{Action, DebugConsoleAction, GlobalAction};
use crate::dispatcher::Dispatcher;
use crate::log_reader::FileLogReader;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::ViewId;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Middleware for managing debug console log reading
pub struct DebugConsoleMiddleware {
    log_file: PathBuf,
    /// Shared reader - accessed by both middleware and polling thread
    reader: Arc<Mutex<Option<FileLogReader>>>,
    /// Flag to signal polling thread to stop
    polling_active: Arc<Mutex<bool>>,
    /// Handle to the polling thread
    poll_thread: Option<JoinHandle<()>>,
}

impl DebugConsoleMiddleware {
    pub fn new(log_file: PathBuf) -> Self {
        Self {
            log_file,
            reader: Arc::new(Mutex::new(None)),
            polling_active: Arc::new(Mutex::new(false)),
            poll_thread: None,
        }
    }

    fn is_console_visible(state: &AppState) -> bool {
        state.active_view().view_id() == ViewId::DebugConsole
    }

    /// Start the polling thread
    fn start_polling(&mut self, dispatcher: &Dispatcher) {
        // Check if already polling
        if *self.polling_active.lock().unwrap() {
            return;
        }

        // Initialize reader and dispatch initial lines
        {
            let mut reader_guard = self.reader.lock().unwrap();
            if reader_guard.is_none() {
                let mut reader = FileLogReader::new(self.log_file.clone(), 10_000);
                match reader.read_initial() {
                    Ok(initial_lines) if !initial_lines.is_empty() => {
                        dispatcher.dispatch(Action::DebugConsole(
                            DebugConsoleAction::LinesUpdated(initial_lines),
                        ));
                    }
                    Err(e) => {
                        log::warn!("Failed to read log file: {}", e);
                    }
                    _ => {}
                }
                *reader_guard = Some(reader);
            }
        }

        // Set polling active flag
        *self.polling_active.lock().unwrap() = true;

        // Spawn polling thread
        let reader = Arc::clone(&self.reader);
        let polling_active = Arc::clone(&self.polling_active);
        let dispatcher = dispatcher.clone();

        self.poll_thread = Some(thread::spawn(move || {
            let poll_interval = Duration::from_millis(400);

            loop {
                // Check if we should stop
                if !*polling_active.lock().unwrap() {
                    break;
                }

                // Poll for new lines (returns only delta)
                let new_lines = {
                    if let Ok(mut reader_guard) = reader.lock() {
                        if let Some(reader) = reader_guard.as_mut() {
                            reader.poll_new_lines().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                // Only dispatch if we have new lines
                if let Some(lines) = new_lines {
                    if !lines.is_empty() {
                        dispatcher.dispatch(Action::DebugConsole(
                            DebugConsoleAction::LinesUpdated(lines),
                        ));
                    }
                }

                thread::sleep(poll_interval);
            }
        }));
    }

    /// Stop the polling thread
    fn stop_polling(&mut self) {
        // Signal thread to stop
        *self.polling_active.lock().unwrap() = false;

        // Wait for thread to finish
        if let Some(handle) = self.poll_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DebugConsoleMiddleware {
    fn drop(&mut self) {
        self.stop_polling();
    }
}

impl Middleware for DebugConsoleMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // When debug console is opened, start polling
            Action::Global(GlobalAction::PushView(view))
                if view.view_id() == ViewId::DebugConsole =>
            {
                self.start_polling(dispatcher);
                true
            }

            // When console is closed, stop polling
            Action::Global(GlobalAction::Close) if Self::is_console_visible(state) => {
                self.stop_polling();
                true
            }

            // Handle clear action
            Action::DebugConsole(DebugConsoleAction::Clear) => {
                if let Ok(mut reader_guard) = self.reader.lock() {
                    if let Some(reader) = reader_guard.as_mut() {
                        reader.clear();
                    }
                }
                // Let action pass through to reducer
                true
            }

            _ => true,
        }
    }
}
