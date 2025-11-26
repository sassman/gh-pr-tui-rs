//! KeyboardMiddleware - translates keyboard events into context-aware actions
//!
//! This middleware intercepts `GlobalKeyPressed` actions and translates them into
//! appropriate navigation/scrolling actions based on:
//! - Which panel is currently active
//! - Multi-key sequences (e.g., "gg" for go-to-top)
//! - Vim-style navigation patterns

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::DebugConsoleView;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

/// KeyboardMiddleware handles vim-style keyboard navigation
///
/// # Features
/// - Context-aware: Different keys do different things based on panel capabilities
/// - Multi-key sequences: "gg" → go to top, "G" → go to bottom
/// - Vim-style: j/k for navigation, h/l for horizontal movement
///
/// # Supported Contexts
/// - **Main View**: j/k navigate items
/// - **Debug Console**: j/k scroll logs, gg/G jump to first/last
pub struct KeyboardMiddleware {
    /// Last key pressed for multi-key sequences
    last_key: Option<(char, Instant)>,
    /// Timeout for multi-key sequences (500ms)
    sequence_timeout: Duration,
}

impl KeyboardMiddleware {
    pub fn new() -> Self {
        Self {
            last_key: None,
            sequence_timeout: Duration::from_millis(500),
        }
    }

    /// Check if we have a pending key that can form a sequence
    fn check_sequence(&mut self, current_key: char) -> Option<KeySequence> {
        if let Some((last_char, last_time)) = self.last_key {
            // Check if timeout expired
            if last_time.elapsed() > self.sequence_timeout {
                self.clear_sequence();
                return None;
            }

            // Check for "gg" sequence (go to top)
            match (last_char, current_key) {
                ('g', 'g') => {
                    self.clear_sequence();
                    return Some(KeySequence::GoToTop);
                }
                (_, _) => {
                    // do nothing
                }
            }
        }

        None
    }

    /// Record a key for potential sequence
    fn record_key(&mut self, key: char) {
        self.last_key = Some((key, Instant::now()));
    }

    /// Clear sequence state
    fn clear_sequence(&mut self) {
        self.last_key = None;
    }

    /// Handle a key event based on panel capabilities
    ///
    /// This is capability-based: it maps keys to semantic actions based on what the
    /// panel declares it supports, without knowing anything about specific panels.
    fn handle_key(
        &mut self,
        key: KeyEvent,
        capabilities: crate::capabilities::PanelCapabilities,
        state: &AppState,
        dispatcher: &Dispatcher,
    ) -> bool {
        use crate::capabilities::PanelCapabilities;

        // Handle character keys (vim-style)
        if let KeyCode::Char(c) = key.code {
            // Handle Ctrl+key combinations
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'c' => {
                        dispatcher.dispatch(Action::GlobalQuit);
                        return false;
                    }
                    'd' if capabilities.contains(PanelCapabilities::VIM_SCROLL_BINDINGS) => {
                        dispatcher.dispatch(Action::ScrollHalfPageDown);
                        return false;
                    }
                    'u' if capabilities.contains(PanelCapabilities::VIM_SCROLL_BINDINGS) => {
                        dispatcher.dispatch(Action::ScrollHalfPageUp);
                        return false;
                    }
                    _ => return true, // Pass through other Ctrl combinations
                }
            }

            // Check for multi-key sequences first
            if let Some(sequence) = self.check_sequence(c) {
                return self.handle_sequence(sequence, capabilities, dispatcher);
            }

            // Handle single character commands based on capabilities
            match c {
                // Vim navigation: j (down) / k (up)
                'j' if capabilities.supports_vim_navigation() => {
                    dispatcher.dispatch(Action::NavigateNext);
                    return false; // Block original KeyPressed action
                }

                'k' if capabilities.supports_vim_navigation() => {
                    dispatcher.dispatch(Action::NavigatePrevious);
                    return false;
                }

                // Vim navigation: h (left) / l (right)
                'h' if capabilities.supports_vim_navigation() => {
                    dispatcher.dispatch(Action::NavigateLeft);
                    return false;
                }

                'l' if capabilities.supports_vim_navigation() => {
                    dispatcher.dispatch(Action::NavigateRight);
                    return false;
                }

                // Go to bottom: G (shift+g)
                'G' => {
                    return self.handle_sequence(KeySequence::GoToBottom, capabilities, dispatcher);
                }

                // Record 'g' for potential "gg" sequence
                'g' => {
                    self.record_key('g');
                    return false; // Wait for second 'g' or timeout
                }

                'q' => {
                    // Dispatch GlobalClose - reducer will handle this contextually
                    // (close panel if panel is open, quit if on main view)
                    dispatcher.dispatch(Action::GlobalClose);
                    return false;
                }

                // Backtick toggles debug console
                '`' => {
                    dispatcher.dispatch(Action::PushView(Box::new(DebugConsoleView::new())));
                    return false;
                }

                // Any other character clears sequence and dispatches as LocalKeyPressed
                _ => {
                    self.clear_sequence();
                    dispatcher.dispatch(Action::LocalKeyPressed(c));
                    return false;
                }
            }
        }

        // Handle Escape key - universal close action
        if let KeyCode::Esc = key.code {
            dispatcher.dispatch(Action::GlobalClose);
            return false;
        }

        // Handle arrow keys based on capabilities
        match key.code {
            KeyCode::Down if capabilities.supports_vim_navigation() => {
                dispatcher.dispatch(Action::NavigateNext);
                return false;
            }

            KeyCode::Up if capabilities.supports_vim_navigation() => {
                dispatcher.dispatch(Action::NavigatePrevious);
                return false;
            }

            KeyCode::Left if capabilities.supports_vim_navigation() => {
                dispatcher.dispatch(Action::NavigateLeft);
                return false;
            }

            KeyCode::Right if capabilities.supports_vim_navigation() => {
                dispatcher.dispatch(Action::NavigateRight);
                return false;
            }

            KeyCode::PageDown if capabilities.contains(PanelCapabilities::SCROLL_VERTICAL) => {
                dispatcher.dispatch(Action::ScrollPageDown);
                return false;
            }

            KeyCode::PageUp if capabilities.contains(PanelCapabilities::SCROLL_VERTICAL) => {
                dispatcher.dispatch(Action::ScrollPageUp);
                return false;
            }

            // All other keys pass through
            _ => {
                self.clear_sequence();
                return true;
            }
        }
    }

    /// Handle a complete key sequence (like "gg")
    ///
    /// This is capability-aware: it checks panel capabilities before dispatching semantic actions
    fn handle_sequence(
        &mut self,
        sequence: KeySequence,
        capabilities: crate::capabilities::PanelCapabilities,
        dispatcher: &Dispatcher,
    ) -> bool {
        match sequence {
            KeySequence::GoToTop => {
                // Only dispatch ScrollToTop if panel supports vim vertical scrolling
                if capabilities.supports_vim_vertical_scroll() {
                    log::debug!(
                        "Dispatching ScrollToTop (capabilities support vim vertical scroll)"
                    );
                    dispatcher.dispatch(Action::ScrollToTop);
                    return false; // Block original key event
                } else {
                    log::debug!("Ignoring 'gg' - panel doesn't support vim vertical scrolling");
                    return true; // Pass through
                }
            }

            KeySequence::GoToBottom => {
                // Only dispatch ScrollToBottom if panel supports vim vertical scrolling
                if capabilities.supports_vim_vertical_scroll() {
                    log::debug!(
                        "Dispatching ScrollToBottom (capabilities support vim vertical scroll)"
                    );
                    dispatcher.dispatch(Action::ScrollToBottom);
                    return false; // Block original key event
                } else {
                    log::debug!("Ignoring 'G' - panel doesn't support vim vertical scrolling");
                    return true; // Pass through
                }
            }
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
            // Get capabilities from the active (top-most) view via the View trait
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

/// Multi-key sequences
#[derive(Debug, Clone, Copy)]
enum KeySequence {
    GoToTop,    // "gg"
    GoToBottom, // "G"
}
