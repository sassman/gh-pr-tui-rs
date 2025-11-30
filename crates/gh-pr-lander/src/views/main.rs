//! Main application view
//!
//! Renders the repository tabs and PR table.

use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::view_models::{
    determine_main_content, MainContentViewModel, PrTableViewModel, RepositoryTabsViewModel,
};
use crate::views::View;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::Line,
    widgets::{Block, Cell, Paragraph, Row, Table, Widget},
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
        render(state, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}

/// Render the main view
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    // Split into repository tabs area and content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Repository tab bar (single row)
            Constraint::Min(0),    // Content area
        ])
        .split(area);

    // Build and render tabs view model
    let tabs_vm = RepositoryTabsViewModel::from_state(state);
    f.render_widget(RepositoryTabsWidget(&tabs_vm), chunks[0]);

    // Determine and render main content
    match determine_main_content(state) {
        MainContentViewModel::Empty(empty_vm) => {
            render_empty_state(&empty_vm, chunks[1], f);
        }
        MainContentViewModel::PrTable => {
            render_pr_table(state, chunks[1], f);
        }
    }
}

/// Render the PR table for the currently selected repository
fn render_pr_table(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;
    let repo_idx = state.main_view.selected_repository;

    let repo = &state.main_view.repositories[repo_idx];
    let repo_data = state.main_view.repo_data.get(&repo_idx).unwrap();

    // Build view model
    let vm = PrTableViewModel::from_repo_data(repo_data, repo, theme);

    // Build block with header
    let status_line = Line::from(vm.header.status_text.clone())
        .style(ratatui::style::Style::default().fg(vm.header.status_color))
        .right_aligned();

    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::QuadrantOutside)
        .border_style(ratatui::style::Style::default().fg(theme.accent_primary))
        .title(vm.header.title.clone())
        .title(status_line);

    // Build header row
    let header_style = ratatui::style::Style::default()
        .fg(theme.accent_primary)
        .add_modifier(Modifier::BOLD);

    let header_cells = ["#PR", "Title", "Author", "Comments", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));

    let header = Row::new(header_cells).height(1);

    // Build rows from view model
    let rows: Vec<Row> = vm
        .rows
        .iter()
        .map(|row_vm| {
            let style = ratatui::style::Style::default()
                .fg(row_vm.fg_color)
                .bg(row_vm.bg_color);

            Row::new(vec![
                Cell::from(row_vm.pr_number.clone()),
                Cell::from(row_vm.title.clone()),
                Cell::from(row_vm.author.clone()),
                Cell::from(row_vm.comments.clone()),
                Cell::from(row_vm.status_text.clone())
                    .style(ratatui::style::Style::default().fg(row_vm.status_color)),
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(6),      // #PR
        Constraint::Percentage(50), // Title
        Constraint::Percentage(15), // Author
        Constraint::Length(10),     // Comments
        Constraint::Percentage(15), // Status
    ];

    // Selected row style
    let selected_style = ratatui::style::Style::default()
        .bg(theme.selected_bg)
        .fg(theme.active_fg)
        .add_modifier(Modifier::BOLD);

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(selected_style)
        .highlight_symbol("> ");

    // Create a table state for highlighting
    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(Some(vm.selected_index));

    f.render_stateful_widget(table, area, &mut table_state);
}

/// Render empty/loading state
fn render_empty_state(
    vm: &crate::view_models::EmptyStateViewModel,
    area: Rect,
    f: &mut Frame,
) {
    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::QuadrantOutside)
        .border_style(ratatui::style::Style::default().fg(vm.border_color));

    let paragraph = Paragraph::new(vm.message.clone())
        .block(block)
        .style(vm.text_style)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

/// Widget wrapper for rendering repository tabs from view model
struct RepositoryTabsWidget<'a>(&'a RepositoryTabsViewModel);

impl Widget for RepositoryTabsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < 10 {
            return;
        }

        let vm = self.0;
        let mut x = area.x + 1; // Start with a small margin

        // Render each tab
        for tab in &vm.tabs {
            if x + tab.width > area.x + area.width {
                break; // Don't overflow
            }

            // Render tab with padding
            let padded_text = format!("  {}  ", tab.display_text);
            buf.set_string(x, area.y, &padded_text, tab.style);

            x += tab.width + 1; // Gap between tabs
        }

        // Render hint at the end
        if x + vm.hint.width <= area.x + area.width {
            buf.set_string(x, area.y, &vm.hint.text, vm.hint.style);
        }
    }
}
