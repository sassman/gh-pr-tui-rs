//! Main application view
//!
//! Renders the repository tabs and PR table.

use crate::actions::{
    Action, AvailableAction, BuildLogAction, ContextAction, DiffViewerAction, NavigationAction,
    PullRequestAction,
};
use crate::capabilities::PanelCapabilities;
use crate::command_id::CommandId;
use crate::state::AppState;
use crate::view_models::{
    determine_main_content, MainContentViewModel, PrTableViewModel, RepositoryTabsViewModel,
    StatusBarViewModel,
};
use crate::views::repository_tabs_view::RepositoryTabsWidget;
use crate::views::status_bar::StatusBarWidget;
use crate::views::View;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table},
    Frame,
};

/// Main application view
#[derive(Debug, Clone)]
pub struct PullRequestView;

impl PullRequestView {
    pub fn new() -> Self {
        Self
    }
}

impl View for PullRequestView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::PullRequestView
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

    fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
        let action = match nav {
            NavigationAction::Next => PullRequestAction::NavigateNext,
            NavigationAction::Previous => PullRequestAction::NavigatePrevious,
            NavigationAction::ToTop => PullRequestAction::NavigateToTop,
            NavigationAction::ToBottom => PullRequestAction::NavigateToBottom,
            NavigationAction::Left | NavigationAction::Right => return None,
        };
        Some(Action::PullRequest(action))
    }

    fn translate_context_action(&self, action: ContextAction, _state: &AppState) -> Option<Action> {
        let pr_action = match action {
            ContextAction::Confirm => PullRequestAction::OpenInBrowser,
            ContextAction::ToggleSelect => PullRequestAction::ToggleSelection,
            ContextAction::SelectAll => PullRequestAction::SelectAll,
            ContextAction::DeselectAll => PullRequestAction::DeselectAll,
        };
        Some(Action::PullRequest(pr_action))
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::PullRequest(_)
                | Action::DiffViewer(DiffViewerAction::Open)
                | Action::BuildLog(BuildLogAction::Open)
                | Action::ViewContext(_)
                | Action::Navigate(_)
                | Action::Global(_)
                | Action::MergeBot(_)
        )
    }

    fn available_actions(&self, _state: &AppState) -> Vec<AvailableAction> {
        vec![
            AvailableAction::primary(CommandId::Confirm, "Open"),
            AvailableAction::primary(CommandId::PrMerge, "Merge"),
            AvailableAction::primary(CommandId::PrOpenBuildLogs, "Build Logs"),
            AvailableAction::primary(CommandId::DiffViewerOpen, "Diffs"),
            AvailableAction::selection(CommandId::ToggleSelect, "Select"),
            AvailableAction::navigation(CommandId::RepositoryNext, "Next Repo"),
        ]
    }
}

/// Render the main view
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    // Split into repository tabs, content area, and status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Repository tab bar (single row)
            Constraint::Min(0),    // Content area
            Constraint::Length(1), // Status bar (single row)
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

    // Render status bar at the bottom
    let status_vm = StatusBarViewModel::from_state(state);
    f.render_widget(StatusBarWidget(&status_vm), chunks[2]);
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
    let header_style = theme.table_header();

    // Column widths: Delta=12, Comments=8
    let header_cells = [
        "  #PR".to_string(),
        "Title".to_string(),
        "Author".to_string(),
        format!("{:^12}", "Delta"),
        format!("{:^8}", "Comments"),
        "Status".to_string(),
    ]
    .into_iter()
    .map(|h| Cell::from(h).style(header_style));

    let header = Row::new(header_cells).style(header_style).height(1);

    // Build rows from view model
    let rows: Vec<Row> = vm
        .rows
        .iter()
        .map(|row_vm| {
            let style = Style::default().fg(row_vm.fg_color).bg(row_vm.bg_color);

            // Build delta cell with colored additions (green) and deletions (red)
            // Right-align additions, space, left-align deletions within 12-char column
            let add_str = format!("+{}", row_vm.additions);
            let del_str = format!("-{}", row_vm.deletions);
            let delta_line = Line::from(vec![
                Span::styled(format!("{:>5}", add_str), Style::default().fg(Color::Green)),
                Span::raw(" "),
                Span::styled(format!("{:<6}", del_str), Style::default().fg(Color::Red)),
            ]);

            Row::new(vec![
                Cell::from(row_vm.pr_number.clone()),
                Cell::from(row_vm.title.clone()),
                Cell::from(row_vm.author.clone()),
                Cell::from(delta_line),
                Cell::from(format!("{:^8}", row_vm.comments)), // Center comments in 8-char column
                Cell::from(row_vm.status_text.clone())
                    .style(Style::default().fg(row_vm.status_color)),
            ])
            .style(style)
            .height(1)
        })
        .collect();

    // Calculate PR number column width based on longest PR number
    // Format is "● #12345" or "  #12345" - find max length across all rows
    let pr_number_width = vm
        .rows
        .iter()
        .map(|row| row.pr_number.chars().count())
        .max()
        .unwrap_or(6) // fallback to 6 if no rows
        .max(5) as u16; // minimum width for "  #PR" header

    let widths = [
        Constraint::Length(pr_number_width), // #PR - dynamic width
        Constraint::Percentage(40),          // Title
        Constraint::Percentage(12),          // Author
        Constraint::Length(12),              // Delta (+123 -456)
        Constraint::Length(8),               // Comments
        Constraint::Percentage(15),          // Status
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(theme.table_selected())
        .highlight_symbol("> ");

    // Create a table state for highlighting
    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(Some(vm.selected_index));

    f.render_stateful_widget(table, area, &mut table_state);
}

/// Render empty/loading state
fn render_empty_state(vm: &crate::view_models::EmptyStateViewModel, area: Rect, f: &mut Frame) {
    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::QuadrantOutside)
        .border_style(ratatui::style::Style::default().fg(vm.border_color));

    let paragraph = Paragraph::new(vm.message.clone())
        .block(block)
        .style(vm.text_style)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}
