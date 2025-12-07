//! Key Bindings Panel View Model
//!
//! Pre-computes presentation data for the key bindings help panel.

use crate::command_id::CommandId;
use crate::keybindings::Keymap;
use crate::state::AppState;
use std::collections::BTreeMap;

/// A single binding row in the panel
#[derive(Debug, Clone)]
pub struct BindingRow {
    /// Key hint (e.g., "j/↓", "Ctrl+P", "p → a")
    pub keys: String,
    /// Description of what the binding does
    pub description: String,
}

/// A section grouping related bindings
#[derive(Debug, Clone)]
pub struct BindingSection {
    /// Category name (e.g., "Navigation", "Pull Request")
    pub category: String,
    /// Bindings in this section
    pub bindings: Vec<BindingRow>,
}

/// Pre-computed footer hints
#[derive(Debug, Clone)]
pub struct KeyBindingsFooterHints {
    /// Scroll hint (e.g., "j/↓/k/↑")
    pub scroll: String,
    /// Close hint (e.g., "?/Esc")
    pub close: String,
}

/// View model for the key bindings help panel
#[derive(Debug, Clone)]
pub struct KeyBindingsPanelViewModel {
    /// Panel title
    pub title: String,
    /// Sections of bindings grouped by category
    pub sections: Vec<BindingSection>,
    /// Footer hints for keyboard shortcuts
    pub footer_hints: KeyBindingsFooterHints,
    /// Current scroll offset
    pub scroll_offset: usize,
    /// Total number of lines (for scroll bounds)
    #[allow(dead_code)]
    pub total_lines: usize,
}

impl KeyBindingsPanelViewModel {
    /// Create a view model from app state
    pub fn from_state(state: &AppState) -> Self {
        let keymap = &state.keymap;
        let scroll_offset = state.key_bindings_panel.scroll_offset;

        // Group bindings by category
        let sections = Self::build_sections(keymap);

        // Calculate total lines (categories + bindings + separators)
        let total_lines: usize = sections
            .iter()
            .map(|s| {
                2 // category header + separator line
                + s.bindings.len()
                + 1 // empty line after section
            })
            .sum();

        // Build footer hints
        let footer_hints = KeyBindingsFooterHints {
            scroll: format!(
                "{}/{}",
                keymap
                    .compact_hint_for_command(CommandId::NavigateNext)
                    .unwrap_or_else(|| "j/↓".to_string()),
                keymap
                    .compact_hint_for_command(CommandId::NavigatePrevious)
                    .unwrap_or_else(|| "k/↑".to_string()),
            ),
            close: keymap
                .compact_hint_for_command(CommandId::KeyBindingsToggleView)
                .map(|h| format!("{}/Esc", h))
                .unwrap_or_else(|| "?/Esc".to_string()),
        };

        Self {
            title: " Keyboard Bindings ".to_string(),
            sections,
            footer_hints,
            scroll_offset,
            total_lines,
        }
    }

    /// Build sections from keymap, grouping by category
    fn build_sections(keymap: &Keymap) -> Vec<BindingSection> {
        // Use BTreeMap for consistent ordering
        let mut by_category: BTreeMap<&'static str, Vec<BindingRow>> = BTreeMap::new();

        for binding in keymap.bindings() {
            // Filter out commands that shouldn't be shown
            if !binding.command.show_in_palette()
                && !matches!(
                    binding.command,
                    CommandId::NavigateNext
                        | CommandId::NavigatePrevious
                        | CommandId::NavigateToTop
                        | CommandId::NavigateToBottom
                )
            {
                continue;
            }

            let category = binding.command.category();
            let row = BindingRow {
                keys: binding.hint.clone(),
                description: binding.command.description().to_string(),
            };

            by_category.entry(category).or_default().push(row);
        }

        let category_order = CommandId::category_order();

        // Build sections in defined order
        let mut sections = Vec::new();

        for category in &category_order {
            if let Some(bindings) = by_category.remove(*category) {
                // Deduplicate bindings by description (combine keys for same action)
                let deduped = Self::deduplicate_bindings(bindings);
                if !deduped.is_empty() {
                    sections.push(BindingSection {
                        category: (*category).to_string(),
                        bindings: deduped,
                    });
                }
            }
        }

        // Add any remaining categories not in the defined order
        for (category, bindings) in by_category {
            let deduped = Self::deduplicate_bindings(bindings);
            if !deduped.is_empty() {
                sections.push(BindingSection {
                    category: category.to_string(),
                    bindings: deduped,
                });
            }
        }

        sections
    }

    /// Deduplicate bindings by description, combining keys
    fn deduplicate_bindings(bindings: Vec<BindingRow>) -> Vec<BindingRow> {
        let mut result: Vec<BindingRow> = Vec::new();

        for binding in bindings {
            if let Some(existing) = result
                .iter_mut()
                .find(|b| b.description == binding.description)
            {
                // Combine keys if not already present
                if !existing.keys.contains(&binding.keys) {
                    existing.keys = format!("{}/{}", existing.keys, binding.keys);
                }
            } else {
                result.push(binding);
            }
        }

        result
    }

    /// Calculate max scroll offset based on visible height
    #[allow(dead_code)]
    pub fn max_scroll(&self, visible_height: usize) -> usize {
        self.total_lines.saturating_sub(visible_height)
    }
}
