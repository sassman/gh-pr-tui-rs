//! KeyboardMiddleware - translates keyboard events into context-aware actions
//!
//! This middleware intercepts `KeyPressed` actions and translates them into
//! appropriate navigation/scrolling actions based on:
//! - Which panel is currently active (PR list, log panel, shortcuts, etc.)
//! - Multi-key sequences (e.g., "gg" for go-to-top)
//! - Vim-style navigation patterns

use super::{BoxFuture, Dispatcher, Middleware};
use crate::{actions::Action, state::AppState};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

/// KeyboardMiddleware handles vim-style keyboard navigation
///
/// # Features
/// - Context-aware: Different keys do different things in different panels
/// - Multi-key sequences: "gg" → go to top, "G" → go to bottom
/// - Vim-style: j/k for navigation, h/l for horizontal scroll
///
/// # Supported Contexts
/// - **PR Table**: j/k navigate PRs, gg/G jump to first/last
/// - **Log Panel**: j/k scroll logs, n jump to next section
/// - **Shortcuts Panel**: j/k scroll shortcuts
/// - **Debug Console**: j/k scroll console
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

    /// Determine which panel/context is currently active
    fn get_active_context(state: &AppState) -> PanelContext {
        // Priority order: popups > log panel > shortcuts > main PR table

        // Command palette (highest priority)
        if state.ui.command_palette.is_some() {
            return PanelContext::CommandPalette;
        }

        // Close PR popup
        if state.ui.close_pr_state.is_some() {
            return PanelContext::ClosePrPopup;
        }

        // Add repo popup
        if state.ui.show_add_repo {
            return PanelContext::AddRepoPopup;
        }

        // Log panel
        if state.log_panel.panel.is_some() {
            // Check job list focus via shared state
            let job_list_focused = state
                .log_panel
                .job_list_focused_shared
                .lock()
                .map(|f| *f)
                .unwrap_or(false);

            if job_list_focused {
                return PanelContext::LogPanelJobList;
            } else {
                return PanelContext::LogPanelLogViewer;
            }
        }

        // Debug console
        if state.debug_console.is_open {
            return PanelContext::DebugConsole;
        }

        // Shortcuts panel
        if state.ui.show_shortcuts {
            return PanelContext::ShortcutsPanel;
        }

        // Default: PR table
        PanelContext::PrTable
    }

    /// Handle a key event based on panel capabilities
    ///
    /// This is capability-based: it maps keys to semantic actions based on what the
    /// panel declares it supports, without knowing anything about specific panels.
    fn handle_key(
        &mut self,
        key: KeyEvent,
        capabilities: crate::capabilities::PanelCapabilities,
        dispatcher: &Dispatcher,
    ) -> bool {
        use crate::capabilities::PanelCapabilities;

        // Handle character keys (vim-style)
        if let KeyCode::Char(c) = key.code {
            // Handle Ctrl+key combinations
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
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

                // Any other character clears sequence and passes through
                _ => {
                    self.clear_sequence();
                    return true;
                }
            }
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
                    log::debug!("Dispatching ScrollToTop (capabilities support vim vertical scroll)");
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
                    log::debug!("Dispatching ScrollToBottom (capabilities support vim vertical scroll)");
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
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            // Only intercept KeyPressed actions
            if let Action::KeyPressed(key) = action {
                let capabilities = state.ui.active_panel_capabilities;
                log::debug!(
                    "KeyboardMiddleware: key={:?}, capabilities={:?}",
                    key, capabilities
                );
                return self.handle_key(*key, capabilities, dispatcher);
            }

            // All other actions pass through
            true
        })
    }
}

/// Multi-key sequences
#[derive(Debug, Clone, Copy)]
enum KeySequence {
    GoToTop,    // "gg"
    GoToBottom, // "G"
}

/// Active panel context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelContext {
    /// Main PR table view
    PrTable,
    /// Log panel - job list (tree view)
    LogPanelJobList,
    /// Log panel - log viewer (text view)
    LogPanelLogViewer,
    /// Shortcuts help panel
    ShortcutsPanel,
    /// Debug console
    DebugConsole,
    /// Command palette
    CommandPalette,
    /// Add repository popup
    AddRepoPopup,
    /// Close PR popup
    ClosePrPopup,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_keyboard_middleware_passthrough() {
        let mut middleware = KeyboardMiddleware::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);
        let state = AppState::default();

        // Non-KeyPressed actions should pass through
        let should_continue = middleware.handle(&Action::Quit, &state, &dispatcher).await;

        assert!(should_continue);
    }

    #[tokio::test]
    async fn test_keyboard_middleware_intercepts_keypressed() {
        let mut middleware = KeyboardMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);
        let state = AppState::default();

        // Create a 'j' key event
        let key_event = KeyEvent::from(KeyCode::Char('j'));

        // Should intercept and dispatch NavigateNext (semantic action)
        // Default state has VIM_NAVIGATION_BINDINGS capability
        let should_continue = middleware
            .handle(&Action::KeyPressed(key_event), &state, &dispatcher)
            .await;

        // Should block the original KeyPressed action
        assert!(!should_continue);

        // Should have dispatched NavigateNext (semantic action, not panel-specific)
        let dispatched_action = rx.try_recv();
        assert!(dispatched_action.is_ok());
        assert!(matches!(
            dispatched_action.unwrap(),
            Action::NavigateNext
        ));
    }

    #[test]
    fn test_sequence_timeout() {
        let mut middleware = KeyboardMiddleware::new();

        // Record 'g'
        middleware.record_key('g');

        // Immediately check for 'g' again - should form sequence
        let sequence = middleware.check_sequence('g');
        assert!(matches!(sequence, Some(KeySequence::GoToTop)));

        // State should be cleared
        assert!(middleware.last_key.is_none());
    }

    #[tokio::test]
    async fn test_capability_based_keybindings() {
        use crate::capabilities::PanelCapabilities;

        let mut middleware = KeyboardMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        // Create state with NO vim navigation capabilities
        let mut state = AppState::default();
        state.ui.active_panel_capabilities = PanelCapabilities::empty();

        let key_event = KeyEvent::from(KeyCode::Char('j'));

        // Should pass through (not intercept) because no capabilities
        let should_continue = middleware
            .handle(&Action::KeyPressed(key_event), &state, &dispatcher)
            .await;

        // Should pass through
        assert!(should_continue);

        // Should NOT have dispatched any action
        let dispatched_action = rx.try_recv();
        assert!(dispatched_action.is_err());
    }

    #[tokio::test]
    async fn test_vim_scroll_capabilities() {
        use crate::capabilities::PanelCapabilities;

        let mut middleware = KeyboardMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        // Create state with vim scroll capabilities
        let mut state = AppState::default();
        state.ui.active_panel_capabilities =
            PanelCapabilities::SCROLL_VERTICAL | PanelCapabilities::VIM_SCROLL_BINDINGS;

        // Test 'G' (go to bottom)
        let key_event = KeyEvent::from(KeyCode::Char('G'));
        let should_continue = middleware
            .handle(&Action::KeyPressed(key_event), &state, &dispatcher)
            .await;

        // Should block the original KeyPressed action
        assert!(!should_continue);

        // Should have dispatched ScrollToBottom
        let dispatched_action = rx.try_recv();
        assert!(dispatched_action.is_ok());
        assert!(matches!(
            dispatched_action.unwrap(),
            Action::ScrollToBottom
        ));
    }
}
