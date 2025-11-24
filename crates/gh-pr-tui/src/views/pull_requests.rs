use ratatui::{
    layout::{Constraint, Margin, Rect},
    prelude::*,
    style::palette::tailwind,
    widgets::*,
};

use crate::App;
use crate::theme::Theme;

/// Render the PR table for the currently selected repository
/// Pure presentation - uses pre-computed view model from state
pub fn render_pr_table(f: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.store.state().theme;

    // Get view model from state
    let view_model = app
        .store
        .state()
        .repos
        .repo_data
        .get(&app.store.state().repos.selected_repo)
        .and_then(|rd| rd.pr_table_view_model.as_ref());

    let Some(vm) = view_model else {
        // No view model yet (e.g., during loading)
        f.render_widget(Paragraph::new("Loading...").centered(), area);
        return;
    };

    // Build block with header and status (from view model)
    let status_line = Line::from(vm.header.status_text.clone())
        .style(Style::default().fg(vm.header.status_color))
        .right_aligned();

    let block = Block::default()
        .title(vm.header.title.clone())
        .title(status_line)
        .borders(Borders::ALL);

    // Build header row
    let header_style = Style::default()
        .fg(theme.table_header_fg)
        .bg(theme.table_header_bg);

    let header_cells = ["#PR", "Description", "Author", "#Comments", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));

    let header = Row::new(header_cells)
        .style(Style::default().bg(theme.table_header_bg))
        .height(1);

    // Active/focused row style
    let selected_row_style = Style::default().bg(theme.active_bg).fg(theme.active_fg);

    // Check if empty
    if vm.rows.is_empty() {
        let message = if vm.header.status_text.contains("Loading") {
            "Loading pull requests..."
        } else if vm.header.status_text.contains("Error") {
            "Error loading data. Press Ctrl+r to retry."
        } else {
            "No pull requests found matching filter"
        };

        let paragraph = Paragraph::new(message)
            .block(block)
            .style(Style::default().fg(theme.text_muted))
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(paragraph, area);
        return;
    }

    // Build rows - simple iteration over pre-computed view models!
    let rows = vm.rows.iter().map(|row_vm| {
        Row::new(vec![
            Cell::from(row_vm.pr_number.clone()),
            Cell::from(row_vm.title.clone()),
            Cell::from(row_vm.author.clone()),
            Cell::from(row_vm.comments.clone()),
            Cell::from(row_vm.status_text.clone()).style(Style::default().fg(row_vm.status_color)),
        ])
        .style(Style::default().fg(row_vm.fg_color).bg(row_vm.bg_color))
        .height(1)
    });

    let widths = [
        Constraint::Percentage(8),  // #PR
        Constraint::Percentage(50), // Description
        Constraint::Percentage(15), // Author
        Constraint::Percentage(10), // #Comments
        Constraint::Percentage(17), // Status
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(selected_row_style);

    // Get mutable reference to the current repo's table state (for ratatui rendering only)
    let table_state = app.table_state_for_rendering();
    f.render_stateful_widget(table, area, table_state);
}

/// Render the close PR popup as a centered floating window
pub fn render_close_pr_popup(f: &mut Frame, area: Rect, comment: &str, theme: &Theme) {
    use ratatui::widgets::{Clear, Wrap};

    // Calculate centered area (50% width, smaller height)
    let popup_width = (area.width * 50 / 100).min(60);
    let popup_height = 8; // Fixed height for the form
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Close Pull Request(s) ")
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

    // Build form content
    let text_lines = vec![
        // Instructions
        Line::from(vec![Span::styled(
            "Edit comment (dependabot PRs will use @dependabot close):",
            Style::default().fg(theme.text_secondary),
        )]),
        Line::from(""),
        // Comment field
        Line::from(vec![
            Span::styled(
                "Comment: ",
                Style::default()
                    .fg(theme.active_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                comment,
                Style::default().fg(theme.active_fg).bg(theme.active_bg),
            ),
        ]),
        Line::from(""),
        Line::from(""),
        // Footer with shortcuts
        Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" submit & close  ", Style::default().fg(theme.text_muted)),
            Span::styled(
                "Esc/x/q",
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" cancel", Style::default().fg(theme.text_muted)),
        ]),
    ];

    // Render content
    let paragraph = Paragraph::new(text_lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(paragraph, inner);
}

/// Render context-sensitive action panel showing available shortcuts
pub fn render_action_panel(f: &mut Frame, app: &App, area: Rect) {
    let repo_data = app.get_current_repo_data();
    let selected_count = repo_data.selected_pr_numbers.len();

    let mut actions: Vec<(String, String, Color)> = Vec::new();

    // If log panel is open, show log panel shortcuts
    if app.store.state().log_panel.panel.is_some() {
        actions.push((
            "↑↓/jk".to_string(),
            "Scroll V".to_string(),
            tailwind::CYAN.c600,
        ));
        actions.push((
            "←→/h".to_string(),
            "Scroll H".to_string(),
            tailwind::CYAN.c600,
        ));
        actions.push((
            "n".to_string(),
            "Next Section".to_string(),
            tailwind::CYAN.c600,
        ));
        actions.push((
            "t".to_string(),
            if app
                .store
                .state()
                .log_panel
                .panel
                .as_ref()
                .map(|p| p.show_timestamps)
                .unwrap_or(false)
            {
                "Hide Timestamps".to_string()
            } else {
                "Show Timestamps".to_string()
            },
            tailwind::CYAN.c600,
        ));
        actions.push(("x/Esc".to_string(), "Close".to_string(), tailwind::RED.c600));
    } else if selected_count > 0 {
        // Highlight merge action when PRs are selected
        actions.push((
            "m".to_string(),
            format!("Merge ({})", selected_count),
            tailwind::GREEN.c700,
        ));

        // Show rebase action for manually selected PRs
        actions.push((
            "r".to_string(),
            format!("Rebase ({})", selected_count),
            tailwind::BLUE.c700,
        ));

        // Show approval action for selected PRs
        actions.push((
            "a".to_string(),
            format!("Approve ({})", selected_count),
            tailwind::EMERALD.c600,
        ));
    } else if !repo_data.prs.is_empty() {
        // When nothing selected, show how to select
        actions.push((
            "Space".to_string(),
            "Select".to_string(),
            tailwind::AMBER.c600,
        ));

        // Check if there are PRs that need rebase - show auto-rebase option
        let prs_needing_rebase = repo_data.prs.iter().filter(|pr| pr.needs_rebase).count();
        if prs_needing_rebase > 0 {
            actions.push((
                "r".to_string(),
                format!("Auto-rebase ({})", prs_needing_rebase),
                tailwind::YELLOW.c600,
            ));
        }
    }

    // Add Enter action when PR(s) are selected or focused
    if !repo_data.prs.is_empty() {
        if selected_count > 0 {
            actions.push((
                "Enter".to_string(),
                format!("Open in Browser ({})", selected_count),
                tailwind::PURPLE.c600,
            ));
        } else if let Some(selected_idx) = repo_data.table_state.selected() {
            actions.push((
                "Enter".to_string(),
                "Open in Browser".to_string(),
                tailwind::PURPLE.c600,
            ));

            // Add "l" action for viewing build logs
            if repo_data.prs.get(selected_idx).is_some() {
                actions.push((
                    "l".to_string(),
                    "View Build Logs".to_string(),
                    tailwind::ORANGE.c600,
                ));
                actions.push((
                    "i".to_string(),
                    "Open in IDE".to_string(),
                    tailwind::INDIGO.c600,
                ));
                actions.push((
                    "a".to_string(),
                    "Approve".to_string(),
                    tailwind::EMERALD.c600,
                ));
            }
        }
    }

    // Always add help shortcut at the end
    actions.push(("?".to_string(), "Help".to_string(), tailwind::SLATE.c600));

    // Helper function to create action spans
    let create_action_spans = |actions: &[(String, String, Color)]| -> Vec<Span> {
        let mut spans = Vec::new();
        for (i, (key, label, bg_color)) in actions.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }

            // Key part (highlighted)
            spans.push(Span::styled(
                format!(" {} ", key),
                Style::default()
                    .fg(app.store.state().theme.selected_fg)
                    .bg(*bg_color)
                    .add_modifier(Modifier::BOLD),
            ));

            // Label part
            spans.push(Span::styled(
                format!(" {} ", label),
                Style::default().fg(app.store.state().repos.colors.row_fg),
            ));
        }
        spans
    };

    // Render actions in full-width panel
    let action_spans = create_action_spans(&actions);
    let action_line = Line::from(action_spans);
    let action_paragraph = Paragraph::new(action_line)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(action_paragraph, area);
}
