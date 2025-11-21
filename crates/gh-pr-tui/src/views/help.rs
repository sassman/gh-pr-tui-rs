use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    prelude::*,
    widgets::*,
};

use crate::App;

/// Render the shortcuts help panel as a centered floating window
/// Pure presentation - uses pre-computed view model
/// Returns the maximum scroll offset
pub fn render_shortcuts_panel(
    f: &mut Frame,
    area: Rect,
    app: &App,
) -> usize {
    // Get view model - if not ready yet, return early
    let Some(ref vm) = app.store.state().ui.shortcuts_panel_view_model else {
        return 0;
    };

    let theme = &app.store.state().theme;
    // Calculate centered area (80% width, 90% height)
    let popup_width = (area.width * 80 / 100).min(100);
    let popup_height = (area.height * 90 / 100).min(40);
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: area.x + popup_x,
        y: area.y + popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area and render background
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Block::default().style(Style::default().bg(theme.bg_panel)),
        popup_area,
    );

    // Calculate inner area and split into content and sticky footer
    let inner = popup_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split inner area: content area and 1-line footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Scrollable content
            Constraint::Length(1), // Sticky footer
        ])
        .split(inner);

    let content_area = chunks[0];
    let footer_area = chunks[1];

    // All content pre-computed in view model

    // Render block with title from view model
    let block = Block::default()
        .borders(Borders::ALL)
        .title(vm.title.clone())
        .title_style(
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(block, popup_area);

    // Render scrollable content (lines and scroll from view model)
    let paragraph = Paragraph::new(vm.content_lines.clone())
        .wrap(Wrap { trim: false })
        .scroll((vm.scroll_offset as u16, 0))
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(paragraph, content_area);

    // Render sticky footer (pre-formatted in view model)
    let footer = Paragraph::new(vm.footer_line.clone())
        .style(Style::default().bg(theme.bg_panel))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(footer, footer_area);

    // Return the max scroll value
    vm.max_scroll
}
