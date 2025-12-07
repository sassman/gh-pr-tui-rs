//! Build Log View
//!
//! Renders the build log panel with tree navigation.

use crate::actions::{Action, AvailableAction, BuildLogAction, ContextAction, NavigationAction};
use crate::capabilities::PanelCapabilities;
use crate::command_id::CommandId;
use crate::state::AppState;
use crate::view_models::{BuildLogRowStyle, BuildLogViewModel, StatusBarViewModel};
use crate::views::status_bar::StatusBarWidget;
use crate::views::{View, ViewId};
use ratatui::{prelude::*, widgets::*};

/// Build log view - displays CI workflow results in a tree
#[derive(Debug, Clone)]
pub struct BuildLogView;

impl BuildLogView {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BuildLogView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for BuildLogView {
    fn view_id(&self) -> ViewId {
        ViewId::BuildLog
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        // Split area to preserve status bar at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Build log content
                Constraint::Length(1), // Status bar (single row)
            ])
            .split(area);

        // Render build log panel in upper area
        let vm = BuildLogViewModel::from_state(&state.build_log, &state.theme);
        render_build_log_panel(f, &vm, &state.theme, chunks[0]);

        // Render status bar at bottom
        let status_vm = StatusBarViewModel::from_state(state);
        f.render_widget(StatusBarWidget(&status_vm), chunks[1]);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        PanelCapabilities::SCROLL_VERTICAL
            | PanelCapabilities::SCROLL_HORIZONTAL
            | PanelCapabilities::VIM_SCROLL_BINDINGS
            | PanelCapabilities::VIM_NAVIGATION_BINDINGS
            | PanelCapabilities::ITEM_NAVIGATION
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
        let action = match nav {
            NavigationAction::Next => BuildLogAction::NavigateDown,
            NavigationAction::Previous => BuildLogAction::NavigateUp,
            NavigationAction::ToTop => BuildLogAction::NavigateToTop,
            NavigationAction::ToBottom => BuildLogAction::NavigateToBottom,
            NavigationAction::Left => BuildLogAction::ScrollLeft,
            NavigationAction::Right => BuildLogAction::ScrollRight,
        };
        Some(Action::BuildLog(action))
    }

    fn translate_context_action(&self, action: ContextAction, _state: &AppState) -> Option<Action> {
        match action {
            // Confirm and ToggleSelect both toggle expand/collapse
            ContextAction::Confirm | ContextAction::ToggleSelect => {
                Some(Action::BuildLog(BuildLogAction::Toggle))
            }
            // SelectAll/DeselectAll don't apply to build logs
            _ => None,
        }
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::BuildLog(_) | Action::ViewContext(_) | Action::Navigate(_) | Action::Global(_)
        )
    }

    fn available_actions(&self, _state: &AppState) -> Vec<AvailableAction> {
        vec![
            AvailableAction::primary(CommandId::Confirm, "Toggle"),
            AvailableAction::primary(CommandId::BuildLogNextError, "Next Error"),
            AvailableAction::navigation(CommandId::NavigateNext, "Down"),
            AvailableAction::navigation(CommandId::GlobalClose, "Close"),
        ]
    }
}

/// Render the build log panel from view model
fn render_build_log_panel(
    f: &mut Frame,
    view_model: &BuildLogViewModel,
    theme: &gh_pr_lander_theme::Theme,
    available_area: Rect,
) {
    // Use Clear widget to completely clear the underlying content
    f.render_widget(Clear, available_area);

    // Then render a solid background to ensure complete coverage
    let background = Block::default().style(Style::default().bg(theme.bg_panel));
    f.render_widget(background, available_area);

    // Handle loading state
    if view_model.is_loading {
        let loading_msg = Paragraph::new("Loading build logs...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Build Logs ")
                    .border_style(Style::default().fg(theme.accent_primary))
                    .style(Style::default().bg(theme.bg_panel)),
            )
            .style(Style::default().fg(theme.text_muted).bg(theme.bg_panel))
            .alignment(Alignment::Center);
        f.render_widget(loading_msg, available_area);
        return;
    }

    // Handle error state
    if let Some(ref error) = view_model.error_message {
        let error_msg = Paragraph::new(format!("Error: {}", error))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Build Logs - Error ")
                    .border_style(Style::default().fg(theme.status_error))
                    .style(Style::default().bg(theme.bg_panel)),
            )
            .style(Style::default().fg(theme.status_error).bg(theme.bg_panel))
            .alignment(Alignment::Center);
        f.render_widget(error_msg, available_area);
        return;
    }

    // Split area into PR header (3 lines) and log content
    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // PR context header
            Constraint::Min(0),    // Log content
        ])
        .split(available_area);

    // Render PR context header
    render_build_log_pr_header(f, view_model, theme, card_chunks[0]);

    // Render log content
    render_build_log_tree(f, view_model, theme, card_chunks[1]);
}

/// Render PR context header
fn render_build_log_pr_header(
    f: &mut Frame,
    view_model: &BuildLogViewModel,
    theme: &gh_pr_lander_theme::Theme,
    area: Rect,
) {
    let pr_header_text = vec![
        Line::from(vec![
            Span::styled(
                view_model.pr_header.number_text.clone(),
                Style::default()
                    .fg(view_model.pr_header.number_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                view_model.pr_header.title.clone(),
                Style::default()
                    .fg(view_model.pr_header.title_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            view_model.pr_header.author_text.clone(),
            Style::default().fg(view_model.pr_header.author_color),
        )),
    ];

    let pr_header = Paragraph::new(pr_header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(theme.bg_panel)),
    );

    f.render_widget(pr_header, area);
}

/// Render the tree view - simple iteration over pre-computed rows
fn render_build_log_tree(
    f: &mut Frame,
    view_model: &BuildLogViewModel,
    theme: &gh_pr_lander_theme::Theme,
    area: Rect,
) {
    let visible_height = area.height.saturating_sub(2) as usize;

    if view_model.rows.is_empty() {
        let empty_msg = Paragraph::new("No build logs found")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Build Logs ")
                    .border_style(Style::default().fg(theme.accent_primary))
                    .style(Style::default().bg(theme.bg_panel)),
            )
            .style(Style::default().fg(theme.text_muted).bg(theme.bg_panel))
            .alignment(Alignment::Center);
        f.render_widget(empty_msg, area);
        return;
    }

    // Build table rows - simple iteration, no complex logic!
    let mut rows = Vec::new();
    let start = view_model.scroll_offset;
    let end = (start + visible_height).min(view_model.rows.len());

    for row_vm in &view_model.rows[start..end] {
        // Apply style based on pre-determined row style
        let style = match row_vm.style {
            BuildLogRowStyle::Normal => {
                if row_vm.is_cursor {
                    Style::default()
                        .fg(theme.text_primary)
                        .bg(theme.selected_bg)
                } else {
                    Style::default().fg(theme.text_primary).bg(theme.bg_panel)
                }
            }
            BuildLogRowStyle::Error => {
                if row_vm.is_cursor {
                    // Use yellow text for selected error lines (for visibility against pink background)
                    Style::default()
                        .fg(theme.active_fg)
                        .bg(theme.selected_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(theme.status_error)
                        .bg(theme.bg_panel)
                        .add_modifier(Modifier::BOLD)
                }
            }
            BuildLogRowStyle::Success => {
                if row_vm.is_cursor {
                    Style::default()
                        .fg(theme.text_primary)
                        .bg(theme.selected_bg)
                } else {
                    Style::default().fg(theme.text_primary).bg(theme.bg_panel)
                }
            }
            BuildLogRowStyle::Selected => Style::default()
                .fg(theme.text_primary)
                .bg(theme.selected_bg),
        };

        // Text is pre-formatted - just display it!
        rows.push(Row::new(vec![Cell::from(row_vm.text.clone())]).style(style));
    }

    let table = Table::new(rows, vec![Constraint::Percentage(100)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                // todo: the navigation hints should be dynamic based on keymap, like in other views done
                .title(
                    " Build Logs | j/k: navigate, Enter: toggle, n/N: next/prev error, Esc: close ",
                )
                .border_style(Style::default().fg(theme.accent_primary))
                .style(Style::default().bg(theme.bg_panel)),
        )
        .style(Style::default().bg(theme.bg_panel));

    f.render_widget(table, area);
}
