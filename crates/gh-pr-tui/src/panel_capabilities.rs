//! Panel capability implementations
//!
//! This module implements the PanelCapabilityProvider trait for each panel state,
//! allowing panels to dynamically declare their capabilities based on their current state.

use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::state::{CommandPaletteState, DebugConsoleState, LogPanelState, ReposState, UiState};

/// Implementation for PR table panel (ReposState)
///
/// The PR table supports:
/// - Vim navigation (j/k) for moving through PRs
/// - Item navigation and selection
impl PanelCapabilityProvider for ReposState {
    fn capabilities(&self) -> PanelCapabilities {
        // PR table always supports vim navigation and item operations
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
            | PanelCapabilities::ITEM_NAVIGATION
            | PanelCapabilities::ITEM_SELECTION
    }
}

/// Implementation for shortcuts panel (UiState when shortcuts visible)
///
/// The shortcuts panel supports:
/// - Vertical scrolling when content is larger than viewport
/// - Vim scroll bindings (gg/G)
/// - Vim navigation (j/k)
impl UiState {
    pub fn shortcuts_panel_capabilities(&self) -> PanelCapabilities {
        let mut caps =
            PanelCapabilities::VIM_NAVIGATION_BINDINGS | PanelCapabilities::VIM_SCROLL_BINDINGS;

        // Check if shortcuts panel has scrollable content
        if self.shortcuts_scroll < self.shortcuts_max_scroll {
            caps |= PanelCapabilities::SCROLL_VERTICAL;
        }

        caps
    }
}

/// Implementation for debug console
///
/// The debug console supports:
/// - Vertical scrolling (logs typically exceed viewport)
/// - Vim scroll bindings (gg/G)
/// - Vim navigation (j/k)
impl PanelCapabilityProvider for DebugConsoleState {
    fn capabilities(&self) -> PanelCapabilities {
        // Debug console always has scrollable content (logs)
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
            | PanelCapabilities::VIM_SCROLL_BINDINGS
            | PanelCapabilities::SCROLL_VERTICAL
    }
}

/// Implementation for log panel
///
/// The log panel supports:
/// - Vim navigation (j/k) in both job list and log viewer modes
/// - Horizontal scrolling in log viewer mode
/// - Vertical scrolling when content is large
/// - Tree navigation (expand/collapse) in job list mode
impl PanelCapabilityProvider for LogPanelState {
    fn capabilities(&self) -> PanelCapabilities {
        // Log panel always supports vim navigation and scrolling
        let mut caps =
            PanelCapabilities::VIM_NAVIGATION_BINDINGS | PanelCapabilities::VIM_SCROLL_BINDINGS;

        // Log panel typically has vertical scrollable content
        caps |= PanelCapabilities::SCROLL_VERTICAL;

        // Log viewer mode supports horizontal scrolling for long lines
        caps |= PanelCapabilities::SCROLL_HORIZONTAL;

        caps
    }
}

/// Implementation for command palette
///
/// The command palette supports:
/// - Vim navigation (j/k) for moving through command list
/// - Item navigation
impl PanelCapabilityProvider for CommandPaletteState {
    fn capabilities(&self) -> PanelCapabilities {
        // Command palette supports vim navigation through commands
        PanelCapabilities::VIM_NAVIGATION_BINDINGS | PanelCapabilities::ITEM_NAVIGATION
    }
}

/// Helper function to determine active panel and get its capabilities
///
/// This is used by the reducer to update active_panel_capabilities when focus changes.
pub fn get_active_panel_capabilities(
    repos: &ReposState,
    log_panel: &LogPanelState,
    ui: &UiState,
    debug_console: &DebugConsoleState,
) -> PanelCapabilities {
    // Priority order: popups > log panel > debug console > shortcuts > PR table

    // Command palette (highest priority)
    if let Some(ref palette) = ui.command_palette {
        return palette.capabilities();
    }

    // Close PR popup - minimal capabilities (no vim navigation, just form input)
    if ui.close_pr_state.is_some() {
        return PanelCapabilities::empty();
    }

    // Add repo popup - minimal capabilities (form input)
    if ui.show_add_repo {
        return PanelCapabilities::empty();
    }

    // Log panel
    if log_panel.panel.is_some() {
        return log_panel.capabilities();
    }

    // Debug console
    if debug_console.is_open {
        return debug_console.capabilities();
    }

    // Shortcuts panel
    if ui.show_shortcuts {
        return ui.shortcuts_panel_capabilities();
    }

    // Default: PR table
    repos.capabilities()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pr_table_capabilities() {
        let repos = ReposState::default();
        let caps = repos.capabilities();

        assert!(caps.supports_vim_navigation());
        assert!(caps.contains(PanelCapabilities::ITEM_NAVIGATION));
        assert!(caps.contains(PanelCapabilities::ITEM_SELECTION));
        assert!(!caps.supports_vim_vertical_scroll()); // No scroll capability by default
    }

    #[test]
    fn test_shortcuts_panel_no_scroll() {
        let ui = UiState {
            shortcuts_scroll: 0,
            shortcuts_max_scroll: 0, // No scrollable content
            ..Default::default()
        };

        let caps = ui.shortcuts_panel_capabilities();
        assert!(caps.supports_vim_navigation());
        assert!(!caps.supports_vim_vertical_scroll()); // Can't scroll if no content
    }

    #[test]
    fn test_shortcuts_panel_with_scroll() {
        let ui = UiState {
            shortcuts_scroll: 0,
            shortcuts_max_scroll: 10, // Has scrollable content
            ..Default::default()
        };

        let caps = ui.shortcuts_panel_capabilities();
        assert!(caps.supports_vim_navigation());
        assert!(caps.supports_vim_vertical_scroll()); // Can scroll
    }

    #[test]
    fn test_log_panel_capabilities() {
        let log_panel = LogPanelState::default();
        let caps = log_panel.capabilities();

        assert!(caps.supports_vim_navigation());
        assert!(caps.supports_vim_vertical_scroll());
        assert!(caps.supports_vim_horizontal_scroll());
    }

    #[test]
    fn test_command_palette_capabilities() {
        let palette = CommandPaletteState {
            input: String::new(),
            filtered_commands: vec![],
            selected_index: 0,
            view_model: None,
        };

        let caps = palette.capabilities();
        assert!(caps.supports_vim_navigation());
        assert!(!caps.supports_vim_vertical_scroll()); // No scrolling in command palette
    }
}
