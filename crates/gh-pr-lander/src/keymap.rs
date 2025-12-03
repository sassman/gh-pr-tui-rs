pub use crate::{
    command_id::CommandId,
    keybindings::{KeyBinding, Keymap},
};

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
        KeyBinding::new("r a", "r → a", RepositoryAdd),
        KeyBinding::new("r o", "r → o", RepositoryOpenInBrowser),
        // Scrolling
        // Note: "gg" and "G" are handled specially in keyboard middleware
        KeyBinding::new("g g", "gg", NavigateToTop),
        KeyBinding::new("G", "G", NavigateToBottom),
        // Debug
        KeyBinding::new("`", "`", DebugToggleConsoleView),
        KeyBinding::new("c", "c", DebugClearLogs),
        // Command palette
        KeyBinding::new("ctrl+p", "Ctrl+P", CommandPaletteOpen),
        // PR Selection
        KeyBinding::new("space", "Space", PrToggleSelection),
        KeyBinding::new("ctrl+a", "Ctrl+A", PrSelectAll),
        KeyBinding::new("u", "u", PrDeselectAll),
        KeyBinding::new("ctrl+r", "Ctrl+R", PrRefresh),
        // PR Operations
        KeyBinding::new("enter", "Enter", PrOpenInBrowser),
        KeyBinding::new("p m", "p -> m", PrMerge),
        KeyBinding::new("p a", "p -> a", PrApprove),
        KeyBinding::new("p c", "p -> c", PrComment),
        KeyBinding::new("p d", "p -> d", PrRequestChanges),
        KeyBinding::new("p x", "p -> x", PrClose),
        KeyBinding::new("p i", "p -> i", PrOpenInIDE),
        KeyBinding::new("p l", "p -> l", PrOpenBuildLogs),
        KeyBinding::new("p r", "p -> r", PrRebase),
        // Filter & Search
        KeyBinding::new("f", "f", PrCycleFilter),
        KeyBinding::new("F", "F", PrClearFilter),
        // Build Log Operations
        KeyBinding::new("b l", "b -> l", BuildLogOpen),
        // Merge Bot
        // KeyBinding::new("M", "M", MergeBotStart),
        // KeyBinding::new("Q", "Q", MergeBotAddToQueue),
        // Help
        KeyBinding::new("?", "?", KeyBindingsToggleView),
        // Build Log (view-specific - will be filtered by middleware)
        // Note: Enter for toggle is handled specially in keyboard_middleware due to
        // conflict with PrOpenInBrowser. These keys are only active when BuildLog view is active.
        KeyBinding::new("n", "n", BuildLogNextError),
        KeyBinding::new("N", "N", BuildLogPrevError),
        KeyBinding::new("t", "t", BuildLogToggleTimestamps),
        KeyBinding::new("e", "e", BuildLogExpandAll),
        KeyBinding::new("E", "E", BuildLogCollapseAll),
        // General
        KeyBinding::new("q", "q", GlobalClose),
        KeyBinding::new("esc", "Esc", GlobalClose),
        KeyBinding::new("ctrl+c", "Ctrl+C", GlobalQuit),
    ];

    Keymap::new(bindings)
}
