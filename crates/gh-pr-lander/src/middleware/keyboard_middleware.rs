//! KeyboardMiddleware - translates keyboard events into context-aware actions
//!
//! This middleware uses a three-layer approach to handle keyboard input:
//!
//! ## Layer 1: Priority Keys
//! Keys that always work regardless of context (Ctrl+C, Esc).
//! These are handled directly before any other processing.
//!
//! ## Layer 2: Capabilities
//! Route keys based on view capabilities. For example, views with TEXT_INPUT
//! capability route character keys to text input rather than keybindings.
//!
//! ## Layer 3: Keymap + Gating
//! Look up keys in the keymap, then check if the active view accepts the action.
//! This prevents actions from "leaking" to reducers when a different view is active.

use crate::actions::{Action, GlobalAction, NavigationAction, TextInputAction};
use crate::dispatcher::Dispatcher;
use crate::keybindings::PendingKey;
use crate::middleware::Middleware;
use crate::state::AppState;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

/// KeyboardMiddleware handles keyboard input using a three-layer approach
///
/// # Layers
/// 1. **Priority keys**: Ctrl+C (quit), Esc (close) - always work
/// 2. **Capabilities**: TEXT_INPUT routes chars to text input
/// 3. **Keymap + Gating**: Look up in keymap, check view accepts action
pub struct KeyboardMiddleware {
    /// Pending key for two-key sequences
    pending_key: Option<PendingKey>,
}

impl KeyboardMiddleware {
    pub fn new() -> Self {
        Self { pending_key: None }
    }

    /// Handle a key event using the three-layer approach
    fn handle_key(&mut self, key: KeyEvent, state: &AppState, dispatcher: &Dispatcher) -> bool {
        let view = state.view_stack.last();
        let capabilities = view.map(|v| v.capabilities(state)).unwrap_or_default();

        // ═══════════════════════════════════════════════════════════════════
        // LAYER 1: Priority keys (always work)
        // ═══════════════════════════════════════════════════════════════════

        // Ctrl+C: Emergency quit - always works
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            log::debug!("Layer 1: Ctrl+C - dispatching Quit");
            dispatcher.dispatch(Action::Global(GlobalAction::Quit));
            return false;
        }

        // Esc: Route based on capabilities
        // - If view accepts text input, send TextInputAction::Escape (view decides: cancel/close)
        // - Otherwise, dispatch Global(Close) to close the view
        if key.code == KeyCode::Esc {
            if capabilities.accepts_text_input() {
                log::debug!("Layer 1: Esc - routing to TextInput::Escape (view has TEXT_INPUT)");
                dispatcher.dispatch(Action::TextInput(TextInputAction::Escape));
            } else {
                log::debug!("Layer 1: Esc - dispatching Close");
                dispatcher.dispatch(Action::Global(GlobalAction::Close));
            }
            return false;
        }

        // ═══════════════════════════════════════════════════════════════════
        // LAYER 2: Capability-based routing
        // ═══════════════════════════════════════════════════════════════════

        if capabilities.accepts_text_input() {
            // Clear any pending sequence when in text input mode
            self.pending_key = None;

            // Route character keys to text input (unless Ctrl/Alt modifier)
            if let KeyCode::Char(c) = key.code {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    log::debug!("Layer 2: TEXT_INPUT - routing char '{}' to TextInput", c);
                    dispatcher.dispatch(Action::TextInput(TextInputAction::Char(c)));
                    return false;
                }

                // Ctrl+U - Unix line kill (clear line)
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'u' {
                    dispatcher.dispatch(Action::TextInput(TextInputAction::ClearLine));
                    return false;
                }
            }

            // Route backspace to text input
            if key.code == KeyCode::Backspace {
                if key.modifiers.contains(KeyModifiers::SUPER) {
                    // Cmd+Backspace on Mac - clear entire line
                    dispatcher.dispatch(Action::TextInput(TextInputAction::ClearLine));
                } else {
                    dispatcher.dispatch(Action::TextInput(TextInputAction::Backspace));
                }
                return false;
            }

            // Enter in text input mode triggers Confirm action
            if key.code == KeyCode::Enter {
                dispatcher.dispatch(Action::TextInput(TextInputAction::Confirm));
                return false;
            }

            // Arrow keys for navigation in text input views that support it
            if capabilities.supports_item_navigation() {
                match key.code {
                    KeyCode::Down => {
                        dispatcher.dispatch(Action::Navigate(NavigationAction::Next));
                        return false;
                    }
                    KeyCode::Up => {
                        dispatcher.dispatch(Action::Navigate(NavigationAction::Previous));
                        return false;
                    }
                    _ => {}
                }
            }

            // Tab for field navigation in text input mode
            match key.code {
                KeyCode::Tab => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        dispatcher.dispatch(Action::Navigate(NavigationAction::Previous));
                    } else {
                        dispatcher.dispatch(Action::Navigate(NavigationAction::Next));
                    }
                    return false;
                }
                KeyCode::BackTab => {
                    dispatcher.dispatch(Action::Navigate(NavigationAction::Previous));
                    return false;
                }
                _ => {}
            }

            // Other keys in text input mode are passed through
            // (they'll go through Layer 3 for Ctrl+ combinations)
        }

        // ═══════════════════════════════════════════════════════════════════
        // LAYER 3: Keymap lookup + Gating
        // ═══════════════════════════════════════════════════════════════════

        // Try keymap matching (handles both single keys and two-key sequences)
        // Returns all matching commands - we'll try each one until one is accepted
        let (command_ids, clear_pending, new_pending) =
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
                "Layer 3: Waiting for second key in sequence (first: {})",
                pending_char
            );
            return false; // Don't process further - waiting for second key
        }

        // If keymap matched, try each command until one is accepted (gating)
        for cmd_id in command_ids {
            let action = cmd_id.to_action();

            // Gating: Check if active view accepts this action
            if let Some(view) = view {
                if view.accepts_action(&action) {
                    log::debug!(
                        "Layer 3: Command {:?} accepted by view, dispatching",
                        cmd_id
                    );
                    dispatcher.dispatch(action);
                    return false;
                } else {
                    log::debug!(
                        "Layer 3: Command {:?} rejected by view {:?}, trying next",
                        cmd_id,
                        view.view_id()
                    );
                }
            } else {
                // No view - just dispatch (shouldn't normally happen)
                dispatcher.dispatch(action);
                return false;
            }
        }

        // Unhandled keys are consumed (not passed through)
        false
    }
}

impl Default for KeyboardMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for KeyboardMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Only intercept Global KeyPressed actions
        if let Action::Global(GlobalAction::KeyPressed(key)) = action {
            log::debug!("KeyboardMiddleware: key={:?}", key);
            return self.handle_key(*key, state, dispatcher);
        }

        // All other actions pass through
        true
    }
}
