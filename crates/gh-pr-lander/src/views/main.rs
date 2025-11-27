use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::theme::Theme;
use crate::views::View;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
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
        // Render the main view content
        render(state, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Main view supports vim navigation
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }
}

/// Render the main view
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    // Create main block
    let block = Block::default()
        .title(" Github PR Lander ")
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title());

    // Split into repository tabs area and content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Repository tab bar
            Constraint::Min(0),    // Content area
        ])
        .split(block.inner(area));

    // Render the outer block
    f.render_widget(block, area);

    // Render DOS-style repository tabs
    let tab_titles = vec!["Repository 1", "Repository 2"];
    let dos_tabs = DosStyleTabs::new(tab_titles, state.main_view.selected_repository, theme);
    f.render_widget(dos_tabs, chunks[0]);

    // Render repository content based on selected repository
    let content = match state.main_view.selected_repository {
        0 => render_repo1_content(theme),
        1 => render_repo2_content(theme),
        _ => vec![Line::from("Invalid repository")],
    };

    let paragraph = Paragraph::new(content)
        .style(theme.panel_background())
        .alignment(Alignment::Center);

    f.render_widget(paragraph, chunks[1]);
}

/// Render content for Repository 1
fn render_repo1_content(theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Repository 1!",
            theme.success().bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This is placeholder content for the first repository",
            theme.text_secondary(),
        )),
        Line::from(""),
        Line::from(Span::styled("Controls:", theme.section_header())),
        Line::from(vec![
            Span::styled("  Tab/Shift+Tab  ", theme.key_hint()),
            Span::styled("- Switch repositories", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  `              ", theme.key_hint()),
            Span::styled("- Toggle debug console", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  q or Esc       ", theme.key_hint()),
            Span::styled("- Quit", theme.key_description()),
        ]),
    ]
}

/// Render content for Repository 2
fn render_repo2_content(theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Repository 2!",
            theme.success().bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This is placeholder content for the second repository",
            theme.text_secondary(),
        )),
        Line::from(""),
        Line::from("More content coming soon..."),
    ]
}

/// DOS/Turbo Vision style tabs widget
/// Renders tabs with box-drawing characters like classic MS-DOS applications
struct DosStyleTabs<'a> {
    titles: Vec<&'a str>,
    selected: usize,
    theme: &'a Theme,
}

impl<'a> DosStyleTabs<'a> {
    fn new(titles: Vec<&'a str>, selected: usize, theme: &'a Theme) -> Self {
        Self {
            titles,
            selected,
            theme,
        }
    }
}

impl Widget for DosStyleTabs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        // DOS-style tab characters
        // Selected tab:   ┌────────┐
        //                 │  Tab   │
        // Unselected:     └────────┴───
        // Bottom line connects selected tab to content

        let mut x = area.x + 1; // Start with a small margin

        // Track tab positions for the bottom line
        let mut tab_positions: Vec<(u16, u16, bool)> = Vec::new(); // (start_x, end_x, is_selected)

        // First pass: render each tab
        for (i, title) in self.titles.iter().enumerate() {
            let is_selected = i == self.selected;
            let tab_width = title.len() as u16 + 4; // 2 chars padding on each side

            if x + tab_width > area.x + area.width {
                break; // Don't overflow
            }

            let start_x = x;
            let end_x = x + tab_width - 1;
            tab_positions.push((start_x, end_x, is_selected));

            // Styles
            let border_style = if is_selected {
                self.theme.panel_border()
            } else {
                self.theme.muted()
            };
            let text_style = if is_selected {
                self.theme.panel_title().bold()
            } else {
                self.theme.text_secondary()
            };
            let bg_style = self.theme.panel_background();

            // Row 0: Top border ┌────────┐
            buf.set_string(x, area.y, "┌", border_style);
            for dx in 1..tab_width - 1 {
                buf.set_string(x + dx, area.y, "─", border_style);
            }
            buf.set_string(x + tab_width - 1, area.y, "┐", border_style);

            // Row 1: Content │  Tab   │
            buf.set_string(x, area.y + 1, "│", border_style);
            // Fill background
            for dx in 1..tab_width - 1 {
                buf.set_string(x + dx, area.y + 1, " ", bg_style);
            }
            // Center the title
            let title_start = x + 2;
            buf.set_string(title_start, area.y + 1, *title, text_style);
            buf.set_string(x + tab_width - 1, area.y + 1, "│", border_style);

            // Row 2: Bottom - handled in second pass
            x += tab_width + 1; // Gap between tabs
        }

        // Second pass: render the bottom line
        // This creates the connected look where selected tab opens into content
        let bottom_y = area.y + 2;
        let border_style = self.theme.panel_border();
        let muted_style = self.theme.muted();

        // Fill the entire bottom row first with the base line
        for dx in 0..area.width {
            buf.set_string(area.x + dx, bottom_y, "─", border_style);
        }

        // Now handle each tab's bottom
        for (start_x, end_x, is_selected) in &tab_positions {
            if *is_selected {
                // Selected tab: open bottom (connects to content)
                // ┘          └
                buf.set_string(*start_x, bottom_y, "┘", border_style);
                for dx in 1..(end_x - start_x) {
                    buf.set_string(start_x + dx, bottom_y, " ", self.theme.panel_background());
                }
                buf.set_string(*end_x, bottom_y, "└", border_style);
            } else {
                // Unselected tab: closed bottom with frame border color
                // └──────────┘
                buf.set_string(*start_x, bottom_y, "└", border_style);
                for dx in 1..(end_x - start_x) {
                    buf.set_string(start_x + dx, bottom_y, "─", border_style);
                }
                buf.set_string(*end_x, bottom_y, "┘", border_style);
            }
        }
    }
}
