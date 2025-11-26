use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::state::AppState;
use crate::state::DebugConsoleState;
use crate::theme::Theme;
use crate::view_models::debug_console_view_model::DebugConsoleViewModel;
use crate::views::View;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Debug console view - shows logs overlaid on main view
#[derive(Debug, Clone)]
pub struct DebugConsoleView;

impl DebugConsoleView {
    pub fn new() -> Self {
        Self
    }
}

impl View for DebugConsoleView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::DebugConsole
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        // Render debug console - this is a floating view so it renders on top
        render(&state.debug_console, &state.theme, area, f);
    }

    fn capabilities(&self, state: &AppState) -> PanelCapabilities {
        // Debug console has its own capabilities
        state.debug_console.capabilities()
    }

    fn is_floating(&self) -> bool {
        // Debug console is a floating view that renders on top of other views
        true
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}

/// Render the debug console (Quake-style drop-down)
fn render(state: &DebugConsoleState, theme: &Theme, area: Rect, f: &mut Frame) {
    if !state.visible {
        return; // Don't render if not visible
    }

    // Calculate console height based on percentage
    let console_height = (area.height * 70) / 100;
    let console_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: console_height.min(area.height),
    };

    f.render_widget(Clear, console_area);

    // Create view model
    let view_model = DebugConsoleViewModel::new(state);

    let block = Block::default()
        .title(view_model.title())
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title());

    // Calculate visible window
    let available_height = console_height.saturating_sub(2) as usize; // -2 for borders

    // Get visible logs and format them
    let visible_logs = view_model.visible_logs(available_height);
    let formatted_lines: Vec<_> = visible_logs
        .iter()
        .map(|record| DebugConsoleViewModel::format_log_line(record, theme))
        .collect();

    let paragraph = Paragraph::new(formatted_lines)
        .block(block)
        .style(theme.panel_background());

    f.render_widget(paragraph, console_area);
}
