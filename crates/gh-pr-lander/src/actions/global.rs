//! Global actions - not tied to any specific screen
//!
//! These actions affect the application as a whole.

use ratatui::crossterm::event::KeyEvent;

use crate::views::View;

/// Global actions that affect the entire application
#[derive(Debug, Clone)]
pub enum GlobalAction {
    /// Raw key pressed (before translation)
    KeyPressed(KeyEvent),
    /// Close the current view (pop from stack)
    Close,
    /// Quit the application
    Quit,
    /// Push a new view onto the stack
    PushView(Box<dyn View>),
    /// Replace entire view stack with new view
    ReplaceView(Box<dyn View>),
    /// Periodic tick for animations
    Tick,
}
