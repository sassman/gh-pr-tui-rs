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
/// - Single char: "q", "a", "1", "G" (case-sensitive for single chars)
/// - With modifiers: "ctrl+p", "shift+tab", "ctrl+shift+c"
/// - Special keys: "tab", "enter", "esc", "backspace", "up", "down", "left", "right"
/// - Two-key sequence: "p a" (space-separated)
pub fn parse_key_pattern(pattern: &str) -> Option<ParsedKeyPattern> {
    let pattern = pattern.trim();

    // Check for two-key sequence (space-separated single chars)
    // Preserve case for character sequences
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

    // For single characters, preserve case (e.g., "G" vs "g")
    // This allows uppercase bindings like "G" for NavigateToBottom
    if pattern.len() == 1 {
        let c = pattern.chars().next()?;
        // Uppercase letters come with SHIFT modifier from terminal
        let modifiers = if c.is_ascii_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        return Some(ParsedKeyPattern::Single {
            code: KeyCode::Char(c),
            modifiers,
        });
    }

    // For everything else (modifiers, special keys), lowercase for matching
    let pattern_lower = pattern.to_lowercase();

    // Parse modifier+key combinations
    let mut modifiers = KeyModifiers::NONE;
    let mut key_part = pattern_lower.as_str();

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
    /// Returns (matched_commands, should_clear_pending, new_pending_key)
    /// Multiple commands can match the same key (e.g., Tab can be RepositoryNext or DiffViewerSwitchPane)
    pub fn match_key(
        &self,
        key: &KeyEvent,
        pending: Option<&PendingKey>,
    ) -> (Vec<CommandId>, bool, Option<char>) {
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
                        return (vec![binding.command], true, None);
                    }
                }
            }
            // Pending key didn't match, clear it and continue to single-key matching
        }

        // Collect all single-key matches
        let mut matches = Vec::new();
        let mut new_pending = None;

        for (binding, pattern) in &self.bindings {
            match pattern {
                ParsedKeyPattern::Single { code, modifiers } => {
                    // Special case: BackTab can come with or without SHIFT modifier
                    // depending on terminal, so we match it loosely
                    let key_matches = if *code == KeyCode::BackTab {
                        key.code == KeyCode::BackTab
                    } else {
                        key.code == *code && key.modifiers == *modifiers
                    };
                    if key_matches {
                        matches.push(binding.command);
                    }
                }
                ParsedKeyPattern::Sequence { first, .. } => {
                    // Check if this key starts a sequence (only if no single-key matches yet)
                    if new_pending.is_none() {
                        if let Some(c) = current_char {
                            if c == *first {
                                new_pending = Some(c);
                            }
                        }
                    }
                }
            }
        }

        // If we have single-key matches, return them
        if !matches.is_empty() {
            return (matches, true, None);
        }

        // If we're starting a sequence, return that
        if let Some(pending) = new_pending {
            return (vec![], false, Some(pending));
        }

        // No match
        (vec![], true, None)
    }

    /// Get all bindings (for displaying in help/command palette)
    pub fn bindings(&self) -> impl Iterator<Item = &KeyBinding> {
        self.bindings.iter().map(|(b, _)| b)
    }

    /// Find the hint for a specific command (returns first match)
    pub fn hint_for_command(&self, command: CommandId) -> Option<&str> {
        self.bindings
            .iter()
            .find(|(b, _)| b.command == command)
            .map(|(b, _)| b.hint.as_str())
    }

    /// Find all hints for a specific command
    pub fn hints_for_command(&self, command: CommandId) -> Vec<&str> {
        self.bindings
            .iter()
            .filter(|(b, _)| b.command == command)
            .map(|(b, _)| b.hint.as_str())
            .collect()
    }

    /// Get a compact hint string for a command (e.g., "j/↓" for NavigateNext)
    /// Deduplicates hints and joins with "/"
    pub fn compact_hint_for_command(&self, command: CommandId) -> Option<String> {
        let hints: Vec<&str> = self
            .bindings
            .iter()
            .filter(|(b, _)| b.command == command)
            .map(|(b, _)| b.hint.as_str())
            .collect();

        if hints.is_empty() {
            return None;
        }

        // Deduplicate (e.g., backtab and shift+tab both have "Shift+Tab" hint)
        let mut unique_hints: Vec<&str> = Vec::new();
        for hint in hints {
            if !unique_hints.contains(&hint) {
                unique_hints.push(hint);
            }
        }

        Some(unique_hints.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keymap() -> Keymap {
        use CommandId::*;
        Keymap::new(vec![
            KeyBinding::new("j", "j", NavigateNext),
            KeyBinding::new("down", "↓", NavigateNext),
            KeyBinding::new("k", "k", NavigatePrevious),
            KeyBinding::new("up", "↑", NavigatePrevious),
            KeyBinding::new("shift+tab", "Shift+Tab", RepositoryPrevious),
            KeyBinding::new("backtab", "Shift+Tab", RepositoryPrevious), // Duplicate hint
            KeyBinding::new("q", "q", GlobalClose),
            KeyBinding::new("esc", "Esc", GlobalClose),
            KeyBinding::new("`", "`", DebugToggleConsoleView),
        ])
    }

    #[test]
    fn test_hint_for_command_returns_first_match() {
        let keymap = test_keymap();

        // Should return first hint for NavigateNext
        assert_eq!(keymap.hint_for_command(CommandId::NavigateNext), Some("j"));

        // Should return first hint for GlobalClose
        assert_eq!(keymap.hint_for_command(CommandId::GlobalClose), Some("q"));
    }

    #[test]
    fn test_hint_for_command_returns_none_for_unmapped() {
        let keymap = test_keymap();

        // CommandPaletteOpen is not in test keymap
        assert_eq!(keymap.hint_for_command(CommandId::CommandPaletteOpen), None);
    }

    #[test]
    fn test_hints_for_command_returns_all_matches() {
        let keymap = test_keymap();

        // NavigateNext has two bindings
        let hints = keymap.hints_for_command(CommandId::NavigateNext);
        assert_eq!(hints, vec!["j", "↓"]);

        // GlobalClose has two bindings
        let hints = keymap.hints_for_command(CommandId::GlobalClose);
        assert_eq!(hints, vec!["q", "Esc"]);
    }

    #[test]
    fn test_hints_for_command_returns_empty_for_unmapped() {
        let keymap = test_keymap();

        let hints = keymap.hints_for_command(CommandId::CommandPaletteOpen);
        assert!(hints.is_empty());
    }

    #[test]
    fn test_compact_hint_joins_with_slash() {
        let keymap = test_keymap();

        // NavigateNext: "j" and "↓" -> "j/↓"
        assert_eq!(
            keymap.compact_hint_for_command(CommandId::NavigateNext),
            Some("j/↓".to_string())
        );

        // NavigatePrevious: "k" and "↑" -> "k/↑"
        assert_eq!(
            keymap.compact_hint_for_command(CommandId::NavigatePrevious),
            Some("k/↑".to_string())
        );
    }

    #[test]
    fn test_compact_hint_deduplicates() {
        let keymap = test_keymap();

        // RepositoryPrevious has two bindings but both have "Shift+Tab" hint
        // Should deduplicate to just "Shift+Tab"
        assert_eq!(
            keymap.compact_hint_for_command(CommandId::RepositoryPrevious),
            Some("Shift+Tab".to_string())
        );
    }

    #[test]
    fn test_compact_hint_returns_none_for_unmapped() {
        let keymap = test_keymap();

        assert_eq!(
            keymap.compact_hint_for_command(CommandId::CommandPaletteOpen),
            None
        );
    }

    #[test]
    fn test_compact_hint_single_binding() {
        let keymap = test_keymap();

        // DebugToggleConsole has only one binding
        assert_eq!(
            keymap.compact_hint_for_command(CommandId::DebugToggleConsoleView),
            Some("`".to_string())
        );
    }

    #[test]
    fn test_uppercase_key_pattern_parsing() {
        // Uppercase "G" should parse with SHIFT modifier
        let pattern = parse_key_pattern("G").unwrap();
        match pattern {
            ParsedKeyPattern::Single { code, modifiers } => {
                assert_eq!(code, KeyCode::Char('G'));
                assert_eq!(modifiers, KeyModifiers::SHIFT);
            }
            _ => panic!("Expected Single pattern"),
        }

        // Lowercase "g" should parse without SHIFT modifier
        let pattern = parse_key_pattern("g").unwrap();
        match pattern {
            ParsedKeyPattern::Single { code, modifiers } => {
                assert_eq!(code, KeyCode::Char('g'));
                assert_eq!(modifiers, KeyModifiers::NONE);
            }
            _ => panic!("Expected Single pattern"),
        }
    }

    #[test]
    fn test_uppercase_key_matching() {
        use CommandId::*;
        let keymap = Keymap::new(vec![
            KeyBinding::new("G", "G", NavigateToBottom),
            KeyBinding::new("g g", "g g", NavigateToTop),
        ]);

        // Shift+G should match "G" binding
        let key = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        let (cmds, _, _) = keymap.match_key(&key, None);
        assert_eq!(cmds, vec![NavigateToBottom]);

        // Lowercase 'g' should start a sequence, not match
        let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        let (cmds, _, pending) = keymap.match_key(&key, None);
        assert!(cmds.is_empty());
        assert_eq!(pending, Some('g'));
    }
}
