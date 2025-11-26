use crate::state::AppState;
use ratatui::{
    layout::{Alignment, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the main view
pub fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    let block = Block::default()
        .title(" gh-pr-lander ")
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title());

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to gh-pr-lander",
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

impl crate::theme::Theme {
    /// Helper method for text_secondary color
    pub fn text_secondary(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.text_secondary)
    }
}
