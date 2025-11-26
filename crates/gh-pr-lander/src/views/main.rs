use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::views::View;
use ratatui::{
    layout::{Alignment, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Main application view
#[derive(Debug, Clone)]
pub struct MainView;

impl MainView {
    pub fn new() -> Self {
        Self
    }
}

impl View for MainView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::Main
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        // Render the main view content
        render(state, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Main view supports vim navigation
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}

/// Render the main view
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    let block = Block::default()
        .title(" Github PR Lander ")
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title());

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Github PR Lander!",
            theme.success().bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "A clean, minimal PR landing tool",
            theme.text_secondary(),
        )),
        Line::from(""),
        Line::from(Span::styled("Controls:", theme.section_header())),
        Line::from(vec![
            Span::styled("  `           ", theme.key_hint()),
            Span::styled("- Toggle debug console", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  j/k or ↓/↑  ", theme.key_hint()),
            Span::styled("- Navigate", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  h/l or ←/→  ", theme.key_hint()),
            Span::styled("- Navigate left/right", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  q or Esc    ", theme.key_hint()),
            Span::styled("- Close/Quit", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C      ", theme.key_hint()),
            Span::styled("- Force quit", theme.key_description()),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(theme.panel_background())
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}
