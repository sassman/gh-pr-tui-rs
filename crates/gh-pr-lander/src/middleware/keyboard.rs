//! KeyboardMiddleware - translates keyboard events into context-aware actions
//!
//! This middleware intercepts `GlobalKeyPressed` actions and translates them into
//! appropriate actions based on:
//! - The keymap (configurable keybindings from AppState)
//! - The capabilities of the active view
//! - Two-key sequences with timeout (e.g., "g g" for scroll-to-top)

use crate::actions::Action;
use crate::capabilities::PanelCapabilities;
use crate::dispatcher::Dispatcher;
use crate::keybindings::PendingKey;
use crate::middleware::Middleware;
use crate::state::AppState;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

/// KeyboardMiddleware handles keyboard input and maps it to actions
///
/// # Features
/// - Keymap-based: All keybindings come from AppState.keymap
/// - Capability-aware: Actions are filtered based on active view capabilities
/// - Two-key sequences: Supports sequences like "g g" or "p a" with timeout
pub struct KeyboardMiddleware {
    /// Pending key for two-key sequences
    pending_key: Option<PendingKey>,
}

impl KeyboardMiddleware {
    pub fn new() -> Self {
        Self { pending_key: None }
    }

    /// Handle a key event
    fn handle_key(
        &mut self,
        key: KeyEvent,
        capabilities: PanelCapabilities,
        state: &AppState,
        dispatcher: &Dispatcher,
    ) -> bool {
        // Views with TEXT_INPUT capability get special handling
        if capabilities.accepts_text_input() {
            return self.handle_text_input_key(key, capabilities, state, dispatcher);
        }

        // Try keymap matching (handles both single keys and two-key sequences)
        let (command_id, clear_pending, new_pending) =
            state.keymap.match_key(&key, self.pending_key.as_ref());

        // Update pending key state
        if clear_pending {
            self.pending_key = None;
        }
        if let Some(pending_char) = new_pending {
            self.pending_key = Some(PendingKey {
                key: pending_char,
                timestamp: Instant::now(),
            });
            log::debug!(
                "Waiting for second key in sequence (first: {})",
                pending_char
            );
            return false; // Don't process further - waiting for second key
        }

        // If keymap matched, dispatch the command's action
        if let Some(cmd_id) = command_id {
            log::debug!("Keymap matched command: {:?}", cmd_id);
            dispatcher.dispatch(cmd_id.to_action());
            return false;
        }

        // For unmatched character keys, dispatch as LocalKeyPressed
        if let KeyCode::Char(c) = key.code {
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
            {
                dispatcher.dispatch(Action::LocalKeyPressed(c));
                return false;
            }
        }

        // Pass through unhandled keys
        true
    }

    /// Handle key events for views that accept text input
    ///
    /// In text input mode:
    /// - Character keys are sent to the input field
    /// - Special keys (Esc, Enter, Backspace, arrows) have their own handling
    /// - Ctrl+C still quits
    fn handle_text_input_key(
        &mut self,
        key: KeyEvent,
        capabilities: PanelCapabilities,
        _state: &AppState,
        dispatcher: &Dispatcher,
    ) -> bool {
        // Clear any pending sequence when in text input mode
        self.pending_key = None;

        match key.code {
            // Escape - context-dependent close behavior
            KeyCode::Esc => {
                dispatcher.dispatch(Action::TextInputEscape);
                false
            }

            // Enter - confirm/execute
            KeyCode::Enter => {
                dispatcher.dispatch(Action::TextInputConfirm);
                false
            }

            // Backspace - remove last character
            KeyCode::Backspace => {
                dispatcher.dispatch(Action::TextInputBackspace);
                false
            }

            // Arrow keys for navigation (if view supports it)
            KeyCode::Down if capabilities.supports_item_navigation() => {
                dispatcher.dispatch(Action::NavigateNext);
                false
            }
            KeyCode::Up if capabilities.supports_item_navigation() => {
                dispatcher.dispatch(Action::NavigatePrevious);
                false
            }

            // Character input
            KeyCode::Char(c) => {
                // Ctrl+C always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    dispatcher.dispatch(Action::GlobalQuit);
                    return false;
                }

                // Send character to text input
                dispatcher.dispatch(Action::TextInputChar(c));
                false
            }

            // Pass through other keys
            _ => true,
        }
    }
}

impl Default for KeyboardMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for KeyboardMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Only intercept GlobalKeyPressed actions
        if let Action::GlobalKeyPressed(key) = action {
            let capabilities = state.active_view().capabilities(state);
            log::debug!(
                "KeyboardMiddleware: key={:?}, capabilities={:?}",
                key,
                capabilities
            );
            return self.handle_key(*key, capabilities, state, dispatcher);
        }

        // All other actions pass through
        true
    }
}
