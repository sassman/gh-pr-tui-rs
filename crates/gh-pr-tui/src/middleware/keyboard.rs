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

    /// Handle a key event in the current context
    ///
    /// Takes both context (for backwards compat) and capabilities (for new semantic actions)
    fn handle_key(
        &mut self,
        key: KeyEvent,
        context: PanelContext,
        capabilities: crate::capabilities::PanelCapabilities,
        dispatcher: &Dispatcher,
    ) -> bool {
        // Handle character keys (vim-style)
        if let KeyCode::Char(c) = key.code {
            // Ignore keys with Ctrl modifier (let them pass through)
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return true;
            }

            // Check for multi-key sequences first
            if let Some(sequence) = self.check_sequence(c) {
                return self.handle_sequence(sequence, capabilities, dispatcher);
            }

            // Handle single character commands
            match c {
                // Vim navigation: j (down) / k (up)
                'j' => {
                    let action = match context {
                        PanelContext::PrTable => Action::NavigateToNextPr,
                        PanelContext::LogPanelJobList => Action::SelectNextJob,
                        PanelContext::LogPanelLogViewer => Action::ScrollLogPanelDown,
                        PanelContext::ShortcutsPanel => Action::ScrollShortcutsDown,
                        PanelContext::DebugConsole => Action::ScrollDebugConsoleDown,
                        PanelContext::CommandPalette => Action::CommandPaletteSelectNext,
                        // Popups don't support vim navigation
                        _ => return true,
                    };
                    dispatcher.dispatch(action);
                    return false; // Block original KeyPressed action
                }

                'k' => {
                    let action = match context {
                        PanelContext::PrTable => Action::NavigateToPreviousPr,
                        PanelContext::LogPanelJobList => Action::SelectPrevJob,
                        PanelContext::LogPanelLogViewer => Action::ScrollLogPanelUp,
                        PanelContext::ShortcutsPanel => Action::ScrollShortcutsUp,
                        PanelContext::DebugConsole => Action::ScrollDebugConsoleUp,
                        PanelContext::CommandPalette => Action::CommandPaletteSelectPrev,
                        // Popups don't support vim navigation
                        _ => return true,
                    };
                    dispatcher.dispatch(action);
                    return false;
                }

                // Horizontal scrolling: h (left) / l (right)
                'h' => {
                    let action = match context {
                        PanelContext::LogPanelLogViewer => Action::ScrollLogPanelLeft,
                        // Only log panel supports horizontal scroll
                        _ => return true,
                    };
                    dispatcher.dispatch(action);
                    return false;
                }

                'l' => {
                    let action = match context {
                        PanelContext::LogPanelLogViewer => Action::ScrollLogPanelRight,
                        // Only log panel supports horizontal scroll
                        _ => return true,
                    };
                    dispatcher.dispatch(action);
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

                // Section navigation: n (next section) - log panel only
                'n' => {
                    if matches!(context, PanelContext::LogPanelLogViewer) {
                        dispatcher.dispatch(Action::NextLogSection);
                        return false;
                    }
                    // Let other contexts pass through
                    return true;
                }

                // Any other character clears sequence and passes through
                _ => {
                    self.clear_sequence();
                    return true;
                }
            }
        }

        // Handle arrow keys
        match key.code {
            KeyCode::Down => {
                let action = match context {
                    PanelContext::PrTable => Action::NavigateToNextPr,
                    PanelContext::LogPanelJobList => Action::SelectNextJob,
                    PanelContext::LogPanelLogViewer => Action::ScrollLogPanelDown,
                    PanelContext::ShortcutsPanel => Action::ScrollShortcutsDown,
                    PanelContext::DebugConsole => Action::ScrollDebugConsoleDown,
                    PanelContext::CommandPalette => Action::CommandPaletteSelectNext,
                    _ => return true,
                };
                dispatcher.dispatch(action);
                return false;
            }

            KeyCode::Up => {
                let action = match context {
                    PanelContext::PrTable => Action::NavigateToPreviousPr,
                    PanelContext::LogPanelJobList => Action::SelectPrevJob,
                    PanelContext::LogPanelLogViewer => Action::ScrollLogPanelUp,
                    PanelContext::ShortcutsPanel => Action::ScrollShortcutsUp,
                    PanelContext::DebugConsole => Action::ScrollDebugConsoleUp,
                    PanelContext::CommandPalette => Action::CommandPaletteSelectPrev,
                    _ => return true,
                };
                dispatcher.dispatch(action);
                return false;
            }

            KeyCode::Left => {
                if matches!(context, PanelContext::LogPanelLogViewer) {
                    dispatcher.dispatch(Action::ScrollLogPanelLeft);
                    return false;
                }
                return true;
            }

            KeyCode::Right => {
                if matches!(context, PanelContext::LogPanelLogViewer) {
                    dispatcher.dispatch(Action::ScrollLogPanelRight);
                    return false;
                }
                return true;
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
                let context = Self::get_active_context(state);
                let capabilities = state.ui.active_panel_capabilities;
                log::debug!(
                    "KeyboardMiddleware: key={:?}, context={:?}, capabilities={:?}",
                    key, context, capabilities
                );
                return self.handle_key(*key, context, capabilities, dispatcher);
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

        // Should intercept and dispatch NavigateToNextPr
        let should_continue = middleware
            .handle(&Action::KeyPressed(key_event), &state, &dispatcher)
            .await;

        // Should block the original KeyPressed action
        assert!(!should_continue);

        // Should have dispatched NavigateToNextPr
        let dispatched_action = rx.try_recv();
        assert!(dispatched_action.is_ok());
        assert!(matches!(
            dispatched_action.unwrap(),
            Action::NavigateToNextPr
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
}
