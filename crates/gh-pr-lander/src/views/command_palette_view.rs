use crate::actions::{
    Action, CommandPaletteAction, ContextAction, NavigationAction, TextInputAction,
};
use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::view_models::CommandPaletteViewModel;
use crate::views::View;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

/// Command palette view - searchable command launcher
#[derive(Debug, Clone)]
pub struct CommandPaletteView;

impl CommandPaletteView {
    pub fn new() -> Self {
        Self
    }
}

impl View for CommandPaletteView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::CommandPalette
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        render(state, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Command palette accepts text input and supports item navigation
        // TEXT_INPUT means character keys go to the input field, not keybindings
        // ITEM_NAVIGATION enables arrow key navigation through results
        PanelCapabilities::TEXT_INPUT | PanelCapabilities::ITEM_NAVIGATION
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
        let action = match nav {
            NavigationAction::Next => CommandPaletteAction::NavigateNext,
            NavigationAction::Previous => CommandPaletteAction::NavigatePrev,
            // Command palette only supports up/down navigation
            NavigationAction::Left
            | NavigationAction::Right
            | NavigationAction::ToTop
            | NavigationAction::ToBottom => return None,
        };
        Some(Action::CommandPalette(action))
    }

    fn translate_text_input(&self, input: TextInputAction) -> Option<Action> {
        let action = match input {
            TextInputAction::Char(c) => CommandPaletteAction::Char(c),
            TextInputAction::Backspace => CommandPaletteAction::Backspace,
            TextInputAction::ClearLine => CommandPaletteAction::Clear,
            TextInputAction::Escape => CommandPaletteAction::Close,
            TextInputAction::Confirm => CommandPaletteAction::Execute,
        };
        Some(Action::CommandPalette(action))
    }

    fn translate_context_action(&self, action: ContextAction, _state: &AppState) -> Option<Action> {
        match action {
            // Confirm executes the selected command
            ContextAction::Confirm => Some(Action::CommandPalette(CommandPaletteAction::Execute)),
            // Other context actions don't apply
            _ => None,
        }
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::CommandPalette(_)
                | Action::ViewContext(_)
                | Action::Navigate(_)
                | Action::TextInput(_)
                | Action::Global(_)
        )
    }
}

/// Render the command palette as a centered floating panel
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    // Build view model - all data preparation happens here
    let vm = CommandPaletteViewModel::from_state(state);

    // Render dimmed overlay over the entire screen to create modal effect
    let overlay = Block::default().style(
        ratatui::style::Style::default()
            .bg(ratatui::style::Color::Black)
            .add_modifier(Modifier::DIM),
    );
    f.render_widget(overlay, area);

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

    // Clear the popup area (removes the dim effect for the popup itself)
    f.render_widget(Clear, popup_area);

    // Render popup background
    f.render_widget(Block::default().style(theme.panel_background()), popup_area);

    // Build footer hint for bottom border using pre-computed hints from view model
    let footer_hint = Line::from(vec![
        Span::styled(" Enter", theme.key_hint().bold()),
        Span::styled(" execute  ", theme.muted()),
        Span::styled(
            format!(
                "{}/{}",
                vm.footer_hints.navigate_up, vm.footer_hints.navigate_down
            ),
            theme.key_hint().bold(),
        ),
        Span::styled(" navigate  ", theme.muted()),
        Span::styled(&vm.footer_hints.close, theme.key_hint().bold()),
        Span::styled(" close ", theme.muted()),
    ]);

    // Render border and title with command count
    let title = format!(" Command Palette ({} commands) ", vm.total_commands);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(theme.panel_title().add_modifier(Modifier::BOLD))
        .title_bottom(footer_hint)
        .title_alignment(Alignment::Center)
        .border_style(theme.panel_border().add_modifier(Modifier::BOLD))
        .style(theme.panel_background());

    f.render_widget(block, popup_area);

    // Calculate inner area with margins
    let inner = popup_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split into input area, results area, and details area (footer is now in bottom border)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(5),    // Results list
            Constraint::Length(2), // Details area
        ])
        .split(inner);

    // Render input box
    let input_text = if vm.input_is_empty {
        Line::from(vec![Span::styled(
            "Type to search commands...",
            theme.muted().italic(),
        )])
    } else {
        Line::from(vec![Span::styled(&vm.input_text, theme.text())])
    };

    let input_paragraph = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.panel_border())
                .style(theme.panel_background()),
        )
        .style(theme.text());

    f.render_widget(input_paragraph, chunks[0]);

    // Render results list
    if vm.visible_rows.is_empty() {
        let no_results = Paragraph::new("No matching commands")
            .style(theme.muted())
            .alignment(Alignment::Center);
        f.render_widget(no_results, chunks[1]);
    } else {
        // Build table rows from pre-computed view model
        let rows: Vec<Row> = vm
            .visible_rows
            .iter()
            .map(|row_vm| {
                // Use pre-computed colors from view model
                let text_style = if row_vm.is_selected {
                    // Yellow text for selected row
                    ratatui::style::Style::default()
                        .fg(row_vm.fg_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    // Normal text color
                    ratatui::style::Style::default().fg(row_vm.fg_color)
                };

                let row_bg = ratatui::style::Style::default().bg(row_vm.bg_color);

                // Combine indicator and shortcut in first cell
                let first_cell = format!("{}{}", row_vm.indicator, row_vm.shortcut_hint);

                Row::new(vec![
                    Cell::from(first_cell).style(text_style),
                    Cell::from(row_vm.title.clone()).style(text_style),
                    Cell::from(row_vm.category.clone()).style(text_style),
                ])
                .style(row_bg)
            })
            .collect();

        let table = Table::new(
            rows,
            vec![
                Constraint::Length(15),                    // Indicator (2) + Shortcut (13)
                Constraint::Percentage(70),                // Title
                Constraint::Length(vm.max_category_width), // Category
            ],
        )
        .style(theme.panel_background());

        f.render_widget(table, chunks[1]);
    }

    // Render details area with selected command description
    if let Some(ref selected_cmd) = vm.selected_command {
        let details_line = Line::from(vec![Span::styled(
            &selected_cmd.description,
            theme.text_secondary(),
        )]);

        let details_paragraph = Paragraph::new(details_line)
            .wrap(Wrap { trim: false })
            .style(theme.panel_background());

        f.render_widget(details_paragraph, chunks[2]);
    }
}
