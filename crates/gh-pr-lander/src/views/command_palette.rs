use crate::capabilities::PanelCapabilities;
use crate::commands::{filter_commands, get_all_commands};
use crate::state::AppState;
use crate::views::View;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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
        // Command palette only supports arrow key navigation (not vim keys)
        // This allows j/k/h/l to be typed into the search field
        PanelCapabilities::empty()
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}

/// Render the command palette as a centered floating panel
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    // Calculate centered area (60% width, 70% height)
    let palette_width = (area.width * 60 / 100).max(50);
    let palette_height = (area.height * 70 / 100).max(20);
    let palette_x = (area.width.saturating_sub(palette_width)) / 2;
    let palette_y = (area.height.saturating_sub(palette_height)) / 2;

    let palette_area = Rect {
        x: area.x + palette_x,
        y: area.y + palette_y,
        width: palette_width,
        height: palette_height,
    };

    // Clear the area behind the palette
    f.render_widget(Clear, palette_area);

    // Create main block
    let block = Block::default()
        .title(" Command Palette ")
        .borders(Borders::ALL)
        .border_style(theme.panel_border().add_modifier(Modifier::BOLD))
        .title_style(theme.panel_title().add_modifier(Modifier::BOLD))
        .style(theme.panel_background());

    // Split into search input area and results area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input
            Constraint::Min(0),    // Results list
        ])
        .split(block.inner(palette_area));

    f.render_widget(block, palette_area);

    // Render search input
    let query_text = if state.command_palette.query.is_empty() {
        Span::styled("Type to search commands...", theme.muted())
    } else {
        Span::styled(&state.command_palette.query, theme.text())
    };

    let search_input = Paragraph::new(Line::from(vec![
        Span::styled("> ", theme.success().bold()),
        query_text,
    ]))
    .style(theme.panel_background())
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme.panel_border()),
    );

    f.render_widget(search_input, chunks[0]);

    // Get all commands and filter by query
    let all_commands = get_all_commands();
    let filtered_commands = filter_commands(&all_commands, &state.command_palette.query);

    // Render command list
    if filtered_commands.is_empty() {
        let no_results = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No commands found",
                theme.muted().italic(),
            )),
        ])
        .alignment(Alignment::Center);
        f.render_widget(no_results, chunks[1]);
    } else {
        // Create list items
        let items: Vec<ListItem> = filtered_commands
            .iter()
            .enumerate()
            .map(|(idx, cmd)| {
                let is_selected = idx == state.command_palette.selected_index;

                let title_style = if is_selected {
                    theme.success().bold()
                } else {
                    theme.text()
                };

                let category_style = if is_selected {
                    theme.text_secondary().bold()
                } else {
                    theme.muted()
                };

                let desc_style = if is_selected {
                    theme.text_secondary()
                } else {
                    theme.muted()
                };

                let content = vec![
                    Line::from(vec![
                        Span::styled(if is_selected { "> " } else { "  " }, theme.success()),
                        Span::styled(&cmd.title, title_style),
                        Span::raw("  "),
                        Span::styled(format!("[{}]", cmd.category), category_style),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(&cmd.description, desc_style),
                    ]),
                ];

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .style(theme.panel_background())
            .highlight_style(theme.panel_background()); // We handle highlighting manually

        f.render_widget(list, chunks[1]);
    }

    // Render footer with hints
    let footer_y = palette_area.y + palette_area.height;
    if footer_y < area.height {
        let footer_area = Rect {
            x: palette_area.x,
            y: footer_y,
            width: palette_width,
            height: 1,
        };

        let hints = Line::from(vec![
            Span::styled("↑/↓", theme.key_hint()),
            Span::styled(" navigate  ", theme.key_description()),
            Span::styled("Enter", theme.key_hint()),
            Span::styled(" execute  ", theme.key_description()),
            Span::styled("Esc", theme.key_hint()),
            Span::styled(" close", theme.key_description()),
        ]);

        let footer = Paragraph::new(hints)
            .style(theme.muted())
            .alignment(Alignment::Center);

        f.render_widget(footer, footer_area);
    }
}
