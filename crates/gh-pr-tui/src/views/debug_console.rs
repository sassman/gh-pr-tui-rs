use ratatui::{prelude::*, widgets::*};

use crate::App;

/// Render the debug console as a Quake-style drop-down panel
/// Pure presentation - uses pre-computed view model with viewport scrolling
pub fn render_debug_console(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::{Clear, List, ListItem};

    let console_state = &app.store.state().debug_console;
    let theme = &app.store.state().theme;

    // Get view model - if not ready yet, return early
    let Some(ref vm) = console_state.view_model else {
        return;
    };

    // Calculate console height based on percentage
    let console_height = (area.height * console_state.height_percent) / 100;
    let console_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: console_height.min(area.height),
    };

    // Clear the area
    f.render_widget(Clear, console_area);

    // Build list items from visible logs (view model already applied scroll offset)
    let log_items: Vec<ListItem> = vm
        .visible_logs
        .iter()
        .map(|log_line| {
            // Text and color are pre-formatted in view model
            ListItem::new(log_line.text.clone()).style(Style::default().fg(log_line.color))
        })
        .collect();

    // Create the list widget (title and footer are pre-formatted)
    let logs_list = List::new(log_items).block(
        Block::bordered()
            .title(vm.title.clone())
            .title_bottom(vm.footer.clone())
            .border_style(Style::default().fg(theme.accent_primary))
            .style(Style::default().bg(theme.bg_secondary)),
    );

    // Render without state - viewport scrolling handled by view model
    f.render_widget(logs_list, console_area);
}
