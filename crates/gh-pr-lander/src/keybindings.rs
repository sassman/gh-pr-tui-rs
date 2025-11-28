//! Keybinding system
//!
//! This module provides the keybinding infrastructure that maps keyboard input
//! to commands. It supports single keys, modifier combinations, and key sequences.
//!
//! # Design
//!
//! - `KeyBinding`: A mapping from a key pattern to a command ID
//! - `KeyPattern`: Textual representation of keys (e.g., "ctrl+p", "p a")
//! - `Keymap`: Collection of bindings with matching logic
//!
//! Key patterns are textual and serializable, allowing future configuration via files.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::command_id::CommandId;

/// A single keybinding that maps a key pattern to a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    /// Textual representation of the key(s) - e.g., "ctrl+p", "p a", "shift+tab"
    pub keys: String,
    /// Display hint for the UI - e.g., "Ctrl+P", "p → a"
    pub hint: String,
    /// The command this binding triggers
    pub command: CommandId,
}

impl KeyBinding {
    /// Create a new keybinding
    pub fn new(keys: impl Into<String>, hint: impl Into<String>, command: CommandId) -> Self {
        Self {
            keys: keys.into(),
            hint: hint.into(),
            command,
        }
    }
}

/// Parsed key pattern for matching
#[derive(Debug, Clone)]
pub enum ParsedKeyPattern {
    /// Single key with optional modifiers
    Single {
        code: KeyCode,
        modifiers: KeyModifiers,
    },
    /// Two-key sequence (e.g., "p a" -> press 'p', then 'a')
    Sequence { first: char, second: char },
}

/// Parse a textual key pattern into a matchable form
///
/// Supported formats:
/// - Single char: "q", "a", "1"
/// - With modifiers: "ctrl+p", "shift+tab", "ctrl+shift+c"
/// - Special keys: "tab", "enter", "esc", "backspace", "up", "down", "left", "right"
/// - Two-key sequence: "p a" (space-separated)
pub fn parse_key_pattern(pattern: &str) -> Option<ParsedKeyPattern> {
    let pattern = pattern.trim().to_lowercase();

    // Check for two-key sequence (space-separated single chars)
    if pattern.contains(' ') {
        let parts: Vec<&str> = pattern.split_whitespace().collect();
        if parts.len() == 2 {
            let first = parts[0].chars().next()?;
            let second = parts[1].chars().next()?;
            if parts[0].len() == 1 && parts[1].len() == 1 {
                return Some(ParsedKeyPattern::Sequence { first, second });
            }
        }
        return None; // Invalid sequence format
    }

    // Parse modifier+key combinations
    let mut modifiers = KeyModifiers::NONE;
    let mut key_part = pattern.as_str();

    // Extract modifiers
    while key_part.contains('+') {
        if let Some((modifier, rest)) = key_part.split_once('+') {
            match modifier {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "alt" => modifiers |= KeyModifiers::ALT,
                _ => break, // Not a modifier, might be the key itself
            }
            key_part = rest;
        } else {
            break;
        }
    }

    // Parse the key code
    let code = parse_key_code(key_part)?;

    Some(ParsedKeyPattern::Single { code, modifiers })
}

/// Parse a key code string into a KeyCode
fn parse_key_code(s: &str) -> Option<KeyCode> {
    match s {
        // Special keys
        "tab" => Some(KeyCode::Tab),
        "backtab" => Some(KeyCode::BackTab),
        "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Esc),
        "backspace" | "bs" => Some(KeyCode::Backspace),
        "delete" | "del" => Some(KeyCode::Delete),
        "insert" | "ins" => Some(KeyCode::Insert),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pageup" | "pgup" => Some(KeyCode::PageUp),
        "pagedown" | "pgdn" => Some(KeyCode::PageDown),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "space" => Some(KeyCode::Char(' ')),

        // Function keys
        s if s.starts_with('f') && s.len() > 1 => {
            let num: u8 = s[1..].parse().ok()?;
            Some(KeyCode::F(num))
        }

        // Single character
        s if s.len() == 1 => {
            let c = s.chars().next()?;
            Some(KeyCode::Char(c))
        }

        _ => None,
    }
}

/// State for tracking pending keys in two-key sequences
#[derive(Debug, Clone)]
pub struct PendingKey {
    pub key: char,
    pub timestamp: Instant,
}

/// The keymap - a collection of keybindings with matching logic
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: Vec<(KeyBinding, ParsedKeyPattern)>,
}

impl Keymap {
    /// Create a new keymap from a list of bindings
    pub fn new(bindings: Vec<KeyBinding>) -> Self {
        let parsed: Vec<_> = bindings
            .into_iter()
            .filter_map(|binding| {
                let pattern = parse_key_pattern(&binding.keys)?;
                Some((binding, pattern))
            })
            .collect();

        Self { bindings: parsed }
    }

    /// Try to match a key event against the keymap
    ///
    /// Returns (matched_command, should_clear_pending, new_pending_key)
    pub fn match_key(
        &self,
        key: &KeyEvent,
        pending: Option<&PendingKey>,
    ) -> (Option<CommandId>, bool, Option<char>) {
        const SEQUENCE_TIMEOUT_SECS: u64 = 2;

        // Get current char if it's a simple char press (no ctrl/alt)
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

        // Check for valid pending key (not timed out)
        let valid_pending =
            pending.filter(|p| p.timestamp.elapsed().as_secs() < SEQUENCE_TIMEOUT_SECS);

        // If we have a pending key, try to complete a sequence
        if let (Some(pending), Some(current)) = (valid_pending, current_char) {
            for (binding, pattern) in &self.bindings {
                if let ParsedKeyPattern::Sequence { first, second } = pattern {
                    if *first == pending.key && *second == current {
                        return (Some(binding.command), true, None);
                    }
                }
            }
            // Pending key didn't match, clear it and continue to single-key matching
        }

        // Try single-key matches
        for (binding, pattern) in &self.bindings {
            match pattern {
                ParsedKeyPattern::Single { code, modifiers } => {
                    if key.code == *code && key.modifiers == *modifiers {
                        return (Some(binding.command), true, None);
                    }
                }
                ParsedKeyPattern::Sequence { first, .. } => {
                    // Check if this key starts a sequence
                    if let Some(c) = current_char {
                        if c == *first {
                            return (None, false, Some(c));
                        }
                    }
                }
            }
        }

        // No match
        (None, true, None)
    }

    /// Get all bindings (for displaying in help/command palette)
    pub fn bindings(&self) -> impl Iterator<Item = &KeyBinding> {
        self.bindings.iter().map(|(b, _)| b)
    }

    /// Find the hint for a specific command
    pub fn hint_for_command(&self, command: CommandId) -> Option<&str> {
        self.bindings
            .iter()
            .find(|(b, _)| b.command == command)
            .map(|(b, _)| b.hint.as_str())
    }
}

/// Get the default keymap
pub fn default_keymap() -> Keymap {
    use CommandId::*;

    let bindings = vec![
        // Navigation
        KeyBinding::new("j", "j", NavigateNext),
        KeyBinding::new("down", "↓", NavigateNext),
        KeyBinding::new("k", "k", NavigatePrevious),
        KeyBinding::new("up", "↑", NavigatePrevious),
        KeyBinding::new("h", "h", NavigateLeft),
        KeyBinding::new("left", "←", NavigateLeft),
        KeyBinding::new("l", "l", NavigateRight),
        KeyBinding::new("right", "→", NavigateRight),
        // Repository
        KeyBinding::new("tab", "Tab", RepositoryNext),
        KeyBinding::new("shift+tab", "Shift+Tab", RepositoryPrevious),
        KeyBinding::new("backtab", "Shift+Tab", RepositoryPrevious),
        KeyBinding::new("p a", "p → a", RepositoryAdd),
        // Scrolling
        KeyBinding::new("ctrl+d", "Ctrl+D", ScrollHalfPageDown),
        KeyBinding::new("ctrl+u", "Ctrl+U", ScrollHalfPageUp),
        KeyBinding::new("pagedown", "PgDn", ScrollPageDown),
        KeyBinding::new("pageup", "PgUp", ScrollPageUp),
        // Note: "gg" and "G" are handled specially in keyboard middleware
        KeyBinding::new("g g", "g g", ScrollToTop),
        KeyBinding::new("G", "G", ScrollToBottom),
        // Debug
        KeyBinding::new("`", "`", DebugToggleConsole),
        // Command palette
        KeyBinding::new("ctrl+p", "Ctrl+P", CommandPaletteOpen),
        // General
        KeyBinding::new("q", "q", GlobalClose),
        KeyBinding::new("esc", "Esc", GlobalClose),
        KeyBinding::new("ctrl+c", "Ctrl+C", GlobalQuit),
    ];

    Keymap::new(bindings)
}
