use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    prelude::*,
    widgets::*,
};

use crate::App;

/// Render the command palette popup
pub fn render_command_palette(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::{Clear, Wrap};

    let palette = match &app.store.state().ui.command_palette {
        Some(p) => p,
        None => return,
    };

    let theme = &app.store.state().theme;

    // Calculate centered area (70% width, 60% height)
    let popup_width = (area.width * 70 / 100).min(100);
    let popup_height = (area.height * 60 / 100).min(30);
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

    // Render border and title
    let title = format!(
        " Command Palette ({} commands) ",
        palette.filtered_commands.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
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

    // Calculate inner area
    let inner = popup_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split into input area, results area, details area, and footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(5),    // Results list
            Constraint::Length(2), // Details area (for selected command)
            Constraint::Length(1), // Footer
        ])
        .split(inner);

    // Render input box
    let input_text = format!("> {}", palette.input);
    let input_paragraph = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent_primary))
                .style(Style::default().bg(theme.bg_secondary)),
        )
        .style(
            Style::default()
                .fg(theme.text_primary)
                .bg(theme.bg_secondary),
        );
    f.render_widget(input_paragraph, chunks[0]);

    // Render results list
    if palette.filtered_commands.is_empty() {
        // No results
        let no_results = Paragraph::new("No matching commands")
            .style(Style::default().fg(theme.text_muted))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(no_results, chunks[1]);
    } else {
        // Calculate visible range
        let visible_height = chunks[1].height as usize;
        let total_items = palette.filtered_commands.len();
        let selected = palette.selected_index;

        // Ensure selected item is visible (scroll if needed)
        let scroll_offset = if selected < visible_height / 2 {
            0
        } else if selected >= total_items.saturating_sub(visible_height / 2) {
            total_items.saturating_sub(visible_height)
        } else {
            selected.saturating_sub(visible_height / 2)
        };

        // Build result lines
        let available_width = chunks[1].width as usize;

        let result_lines: Vec<Line> = palette
            .filtered_commands
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(i, (cmd, _score))| {
                let is_selected = i == selected;

                let mut spans = Vec::new();

                // Selection indicator (2 chars)
                if is_selected {
                    spans.push(Span::styled(
                        "> ",
                        Style::default()
                            .fg(theme.accent_primary)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::raw("  "));
                }

                // Shortcut hint (13 chars: 12 for hint + 1 space)
                if let Some(ref hint) = cmd.shortcut_hint {
                    let hint_text = format!("{:12} ", hint);
                    spans.push(Span::styled(
                        hint_text,
                        Style::default().fg(if is_selected {
                            theme.accent_primary
                        } else {
                            theme.text_muted
                        }),
                    ));
                } else {
                    spans.push(Span::raw("             "));
                }

                // Calculate available width for title
                // Total width - indicator(2) - shortcut(13) - category(len+2 for brackets) - padding(3)
                let category_text = format!("[{}]", cmd.category);
                let fixed_width = 2 + 13 + category_text.len() + 3;
                let max_title_width = available_width.saturating_sub(fixed_width);

                // Truncate title if needed
                let title_text = if cmd.title.len() > max_title_width && max_title_width > 3 {
                    format!("{}...", &cmd.title[..max_title_width.saturating_sub(3)])
                } else {
                    cmd.title.clone()
                };

                spans.push(Span::styled(
                    title_text.clone(),
                    Style::default()
                        .fg(if is_selected {
                            theme.selected_fg
                        } else {
                            theme.text_primary
                        })
                        .bg(if is_selected {
                            theme.selected_bg
                        } else {
                            Color::Reset
                        })
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ));

                // Calculate padding to right-align category
                let used_width = 2 + 13 + title_text.len() + category_text.len();
                let padding = if available_width > used_width {
                    available_width.saturating_sub(used_width)
                } else {
                    1
                };

                spans.push(Span::raw(" ".repeat(padding)));

                // Category (right-aligned)
                spans.push(Span::styled(
                    category_text,
                    Style::default().fg(if is_selected {
                        theme.text_secondary
                    } else {
                        theme.text_muted
                    }),
                ));

                Line::from(spans)
            })
            .collect();

        let results_paragraph = Paragraph::new(result_lines)
            .wrap(Wrap { trim: false })
            .style(Style::default().bg(theme.bg_panel));
        f.render_widget(results_paragraph, chunks[1]);
    }

    // Render details area with selected command info
    if let Some((selected_cmd, _)) = palette.filtered_commands.get(palette.selected_index) {
        let mut details_text = vec![];

        // Show description
        details_text.push(Span::styled(
            selected_cmd.description.clone(),
            Style::default().fg(theme.text_secondary),
        ));

        // Show context if present
        if let Some(ref context) = selected_cmd.context {
            details_text.push(Span::styled(
                format!("  ({})", context),
                Style::default()
                    .fg(theme.text_muted)
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        let details_line = Line::from(details_text);
        let details_paragraph = Paragraph::new(details_line)
            .wrap(Wrap { trim: false })
            .style(Style::default().bg(theme.bg_panel));
        f.render_widget(details_paragraph, chunks[2]);
    }

    // Render footer with keyboard hints
    let footer_line = Line::from(vec![
        Span::styled(
            "Enter",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" execute  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "j/k",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" navigate  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" close", Style::default().fg(theme.text_muted)),
    ]);

    let footer = Paragraph::new(footer_line)
        .style(Style::default().fg(theme.text_secondary))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(footer, chunks[3]);
}
