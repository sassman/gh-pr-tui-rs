use crate::state::DebugConsoleState;
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Render the debug console (Quake-style drop-down)
pub fn render(state: &DebugConsoleState, theme: &Theme, area: Rect, f: &mut Frame) {
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

    let block = Block::default()
        .title(" Debug Console (` to toggle, c to clear) ")
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title());

    // Show last N logs that fit in the console
    let available_height = console_height.saturating_sub(2) as usize; // -2 for borders
    let start_index = state.logs.len().saturating_sub(available_height);

    // Color-code log messages by level
    let visible_logs: Vec<Line> = state.logs[start_index..]
        .iter()
        .map(|log| {
            // Parse log level from message format: "[LEVEL] message"
            if let Some(level_end) = log.find(']') {
                if log.starts_with('[') {
                    let level = &log[1..level_end];
                    let message = &log[level_end + 1..];

                    let style = match level {
                        "ERROR" => theme.log_error(),
                        "WARN" => theme.log_warning(),
                        "INFO" => theme.log_info(),
                        "DEBUG" => theme.log_debug(),
                        _ => theme.text(),
                    };

                    return Line::from(vec![
                        Span::styled(format!("[{}]", level), style.bold()),
                        Span::styled(message, theme.text()),
                    ]);
                }
            }

            // Fallback: no level detected, use default style
            Line::from(Span::styled(log.clone(), theme.text()))
        })
        .collect();

    let paragraph = Paragraph::new(visible_logs)
        .block(block)
        .style(theme.panel_background());

    f.render_widget(paragraph, console_area);
}
