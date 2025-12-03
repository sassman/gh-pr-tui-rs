//! Command Palette State

/// Command palette state
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub query: String,         // Search query
    pub selected_index: usize, // Currently selected command index
}
