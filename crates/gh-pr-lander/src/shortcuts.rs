//! Keyboard shortcut system
//!
//! This module provides keyboard shortcut matching that maps key events to actions.
//! It supports both single-key shortcuts and two-key combinations (like "p → a").

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::actions::Action;

/// Shortcut definition with key matching capability
#[derive(Debug, Clone)]
pub struct Shortcut {
    /// Display string for the shortcut (e.g., "p → a", "Ctrl+P")
    pub key_display: &'static str,
    /// Description of what the shortcut does
    pub description: &'static str,
    /// The action to execute when the shortcut is triggered
    pub action: Action,
    /// The matcher that determines if a key event matches this shortcut
    pub matcher: ShortcutMatcher,
}

/// Matcher for shortcuts - can be single key or two-key combination
#[derive(Clone)]
pub enum ShortcutMatcher {
    /// Single key press (with optional modifiers)
    SingleKey(fn(&KeyEvent) -> bool),
    /// Two-key combination: first_key then second_key (e.g., 'p' then 'a')
    TwoKey(char, char),
}

impl std::fmt::Debug for ShortcutMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortcutMatcher::SingleKey(_) => write!(f, "SingleKey"),
            ShortcutMatcher::TwoKey(k1, k2) => write!(f, "TwoKey({}, {})", k1, k2),
        }
    }
}

impl Shortcut {
    /// Check if this shortcut matches the given key event (for single-key shortcuts)
    pub fn matches(&self, key: &KeyEvent) -> bool {
        match &self.matcher {
            ShortcutMatcher::SingleKey(func) => func(key),
            ShortcutMatcher::TwoKey(_, _) => false, // Two-key shortcuts don't match single key
        }
    }

    /// Check if this is a two-key shortcut starting with the given first key
    pub fn is_two_key_starting_with(&self, first_key: char) -> bool {
        match &self.matcher {
            ShortcutMatcher::TwoKey(k1, _) => *k1 == first_key,
            _ => false,
        }
    }

    /// Check if this two-key shortcut completes with the given second key
    pub fn completes_two_key_with(&self, second_key: char) -> bool {
        match &self.matcher {
            ShortcutMatcher::TwoKey(_, k2) => *k2 == second_key,
            _ => false,
        }
    }
}

/// Get all shortcut definitions
pub fn get_shortcuts() -> Vec<Shortcut> {
    vec![
        // Navigation
        Shortcut {
            key_display: "j / ↓",
            description: "Navigate down",
            action: Action::NavigateNext,
            matcher: ShortcutMatcher::SingleKey(|key| {
                matches!(key.code, KeyCode::Char('j') | KeyCode::Down)
            }),
        },
        Shortcut {
            key_display: "k / ↑",
            description: "Navigate up",
            action: Action::NavigatePrevious,
            matcher: ShortcutMatcher::SingleKey(|key| {
                matches!(key.code, KeyCode::Char('k') | KeyCode::Up)
            }),
        },
        Shortcut {
            key_display: "Tab",
            description: "Next repository",
            action: Action::RepositoryNext,
            matcher: ShortcutMatcher::SingleKey(|key| {
                matches!(key.code, KeyCode::Tab) && !key.modifiers.contains(KeyModifiers::SHIFT)
            }),
        },
        Shortcut {
            key_display: "Shift+Tab",
            description: "Previous repository",
            action: Action::RepositoryPrevious,
            matcher: ShortcutMatcher::SingleKey(|key| {
                matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
                    && key.modifiers.contains(KeyModifiers::SHIFT)
                    || matches!(key.code, KeyCode::BackTab)
            }),
        },
        // Command palette
        Shortcut {
            key_display: "Ctrl+P",
            description: "Open command palette",
            action: Action::PushView(Box::new(crate::views::CommandPaletteView::new())),
            matcher: ShortcutMatcher::SingleKey(|key| {
                matches!(key.code, KeyCode::Char('p'))
                    && key.modifiers.contains(KeyModifiers::CONTROL)
            }),
        },
        // Repository management (two-key combos)
        Shortcut {
            key_display: "p → a",
            description: "Add repository",
            action: Action::RepositoryAdd,
            matcher: ShortcutMatcher::TwoKey('p', 'a'),
        },
        // Debug
        Shortcut {
            key_display: "` / ~",
            description: "Toggle debug console",
            action: Action::PushView(Box::new(crate::views::DebugConsoleView::new())),
            matcher: ShortcutMatcher::SingleKey(|key| {
                matches!(key.code, KeyCode::Char('`') | KeyCode::Char('~'))
            }),
        },
        // General
        Shortcut {
            key_display: "q",
            description: "Quit / Close",
            action: Action::GlobalClose,
            matcher: ShortcutMatcher::SingleKey(|key| matches!(key.code, KeyCode::Char('q'))),
        },
        Shortcut {
            key_display: "Esc",
            description: "Close current view",
            action: Action::GlobalClose,
            matcher: ShortcutMatcher::SingleKey(|key| matches!(key.code, KeyCode::Esc)),
        },
    ]
}

/// Pending key state for two-key combinations
#[derive(Debug, Clone)]
pub struct PendingKeyPress {
    pub key: char,
    pub timestamp: std::time::Instant,
}

/// Find the action for a given key event, handling two-key combinations
///
/// Returns (action, should_clear_pending_key, new_pending_key)
pub fn find_action_for_key(
    key: &KeyEvent,
    pending_key: Option<&PendingKeyPress>,
) -> (Action, bool, Option<char>) {
    const TWO_KEY_TIMEOUT_SECS: u64 = 2;

    // Get the current character if it's a simple char press (no ctrl/alt)
    let current_char = if let KeyCode::Char(c) = key.code {
        if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
        {
            Some(c)
        } else {
            None
        }
    } else {
        None
    };

    // Check if we have a valid pending key (not timed out)
    let valid_pending =
        pending_key.filter(|p| p.timestamp.elapsed().as_secs() < TWO_KEY_TIMEOUT_SECS);

    // If we have a valid pending key, try to complete a two-key combination
    if let (Some(pending), Some(current)) = (valid_pending, current_char) {
        for shortcut in get_shortcuts() {
            if shortcut.is_two_key_starting_with(pending.key)
                && shortcut.completes_two_key_with(current)
            {
                // Two-key combination matched!
                return (shortcut.action.clone(), true, None);
            }
        }
        // Pending key didn't match with current, clear it and process current key normally
        return (find_single_key_action(key), true, None);
    }

    // No valid pending key - check if current key starts a two-key combination
    if let Some(current) = current_char {
        for shortcut in get_shortcuts() {
            if shortcut.is_two_key_starting_with(current) {
                // This key starts a two-key combination - save it as pending
                return (Action::None, false, Some(current));
            }
        }
    }

    // Not a two-key combo - process as single key
    (find_single_key_action(key), true, None)
}

/// Find action for a single key press (no two-key combination logic)
fn find_single_key_action(key: &KeyEvent) -> Action {
    for shortcut in get_shortcuts() {
        if shortcut.matches(key) {
            return shortcut.action.clone();
        }
    }
    Action::None
}
