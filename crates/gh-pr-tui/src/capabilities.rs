//! Panel capability system
//!
//! This module defines capabilities that panels can declare, allowing the keybinding
//! system to make intelligent decisions about which actions are available.
//!
//! For example, vim-style scrolling (G/gg) only makes sense on panels that:
//! - Can scroll vertically
//! - Have vim scroll bindings enabled
//!
//! This keeps the keybinding logic capability-based rather than hardcoded to specific panels.

use bitflags::bitflags;

bitflags! {
    /// Capabilities that a panel can declare
    ///
    /// These are independent of the specific panel type and describe what
    /// operations the panel supports.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PanelCapabilities: u32 {
        /// Panel can scroll vertically (has content that extends beyond viewport)
        const SCROLL_VERTICAL = 1 << 0;

        /// Panel can scroll horizontally (has content that extends beyond viewport)
        const SCROLL_HORIZONTAL = 1 << 1;

        /// Panel supports vim-style scrolling keybindings (gg, G, etc.)
        const VIM_SCROLL_BINDINGS = 1 << 2;

        /// Panel supports vim-style navigation keybindings (j, k, h, l)
        const VIM_NAVIGATION_BINDINGS = 1 << 3;

        /// Panel can navigate to next/previous items
        const ITEM_NAVIGATION = 1 << 4;

        /// Panel supports selection of items
        const ITEM_SELECTION = 1 << 5;
    }
}

impl PanelCapabilities {
    /// Check if panel supports vim-style vertical scrolling
    ///
    /// Requires both vertical scroll capability and vim bindings enabled
    pub fn supports_vim_vertical_scroll(self) -> bool {
        self.contains(Self::SCROLL_VERTICAL | Self::VIM_SCROLL_BINDINGS)
    }

    /// Check if panel supports vim-style horizontal scrolling
    ///
    /// Requires both horizontal scroll capability and vim bindings enabled
    pub fn supports_vim_horizontal_scroll(self) -> bool {
        self.contains(Self::SCROLL_HORIZONTAL | Self::VIM_SCROLL_BINDINGS)
    }

    /// Check if panel supports vim-style navigation (j/k/h/l)
    pub fn supports_vim_navigation(self) -> bool {
        self.contains(Self::VIM_NAVIGATION_BINDINGS)
    }

    /// Check if panel supports item navigation
    pub fn supports_item_navigation(self) -> bool {
        self.contains(Self::ITEM_NAVIGATION)
    }
}

impl Default for PanelCapabilities {
    fn default() -> Self {
        Self::empty()
    }
}

/// Trait for panel state types that can declare their capabilities
///
/// Any panel that wants to support capability-based keybindings should implement this.
/// The implementation should check the panel's current state (content size, config, etc.)
/// and return the appropriate capabilities.
pub trait PanelCapabilityProvider {
    /// Get the current capabilities of this panel
    ///
    /// This should be recomputed whenever the panel's state changes in a way that
    /// affects capabilities (e.g., content size changes, config changes)
    fn capabilities(&self) -> PanelCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vim_vertical_scroll_requires_both_flags() {
        let no_caps = PanelCapabilities::empty();
        assert!(!no_caps.supports_vim_vertical_scroll());

        let only_scroll = PanelCapabilities::SCROLL_VERTICAL;
        assert!(!only_scroll.supports_vim_vertical_scroll());

        let only_vim = PanelCapabilities::VIM_SCROLL_BINDINGS;
        assert!(!only_vim.supports_vim_vertical_scroll());

        let both = PanelCapabilities::SCROLL_VERTICAL | PanelCapabilities::VIM_SCROLL_BINDINGS;
        assert!(both.supports_vim_vertical_scroll());
    }

    #[test]
    fn test_vim_navigation_independent() {
        let vim_nav = PanelCapabilities::VIM_NAVIGATION_BINDINGS;
        assert!(vim_nav.supports_vim_navigation());

        // Vim navigation doesn't require scroll capabilities
        assert!(!vim_nav.supports_vim_vertical_scroll());
    }
}
