use crate::state::AppState;
use ratatui::{layout::Rect, Frame};

pub mod debug_console_view;
pub mod main_view;

/// Render the entire application UI
pub fn render(state: &AppState, area: Rect, f: &mut Frame) {
    // Render base view
    main_view::render(state, area, f);

    // Render debug console on top if visible
    debug_console_view::render(&state.debug_console, &state.theme, area, f);
}
