use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    prelude::*,
    style::palette::tailwind,
    widgets::*,
};

use crate::App;

/// Render the fancy splash screen shown during application bootstrap
/// Pure presentation - uses pre-computed view model
pub fn render_splash_screen(f: &mut Frame, app: &App) {
    // Get view model - if not ready yet, return early
    let Some(ref vm) = app.store.state().infrastructure.splash_screen_view_model else {
        return;
    };

    let area = f.area();

    // Calculate a centered area for the splash screen content
    let centered_area = {
        let width = 50.min(area.width);
        let height = 12.min(area.height);
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        Rect {
            x,
            y,
            width,
            height,
        }
    };

    // Clear background
    f.render_widget(
        Block::default().style(Style::default().bg(tailwind::SLATE.c950)),
        area,
    );

    // Render the centered content box
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(tailwind::BLUE.c500))
        .style(Style::default().bg(tailwind::SLATE.c900));

    f.render_widget(block, centered_area);

    // Split the centered area into sections for content
    let inner = centered_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Title underline
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Spinner
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Progress bar
            Constraint::Length(1), // Spacing
            Constraint::Min(2),    // Status message
        ])
        .split(inner);

    // All stage info pre-computed in view model

    // Title (text and color from view model)
    let title = Paragraph::new(vm.title.clone())
        .style(
            Style::default()
                .fg(vm.title_color)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Title underline
    let underline = Paragraph::new("─".repeat(vm.title.len()))
        .style(Style::default().fg(tailwind::BLUE.c600))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(underline, chunks[1]);

    // Spinner or error icon (pre-formatted in view model)
    let spinner_widget = Paragraph::new(vm.spinner_text.clone())
        .style(
            Style::default()
                .fg(vm.spinner_color)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(spinner_widget, chunks[3]);

    // Progress bar (pre-formatted in view model)
    if !vm.is_error {
        let bar_widget = Paragraph::new(vm.progress_bar.clone())
            .style(Style::default().fg(vm.progress_bar_color))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(bar_widget, chunks[5]);
    }

    // Status message (text and color from view model)
    let message_widget = Paragraph::new(vm.stage_message.clone())
        .style(Style::default().fg(vm.message_color))
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(message_widget, chunks[7]);
}
