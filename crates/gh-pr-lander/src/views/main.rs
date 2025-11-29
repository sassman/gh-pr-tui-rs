use crate::capabilities::PanelCapabilities;
use crate::state::{AppState, Repository};
use crate::theme::Theme;
use crate::views::View;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
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

    // Split into repository tabs area and content area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Repository tab bar (single row)
            Constraint::Min(0),    // Content area with QuadrantOutside border
        ])
        .split(area);

    // Generate tab titles from repositories: "org/repo@branch"
    let tab_titles: Vec<String> = state
        .main_view
        .repositories
        .iter()
        .map(|repo| format!("{}/{}@{}", repo.org, repo.repo, repo.branch))
        .collect();
    let tabs_widget = ModernTabs::new(tab_titles, state.main_view.selected_repository, theme);
    f.render_widget(tabs_widget, chunks[0]);

    // Render content with QuadrantOutside border - uses half-block characters
    // that create a connected appearance with the tab bar
    let content_block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::QuadrantOutside)
        .border_style(ratatui::style::Style::default().fg(theme.accent_primary));

    // Render repository content based on selected repository
    let content = if let Some(repo) = state
        .main_view
        .repositories
        .get(state.main_view.selected_repository)
    {
        render_repo_content(repo, theme)
    } else {
        render_no_repos_content(theme)
    };

    let paragraph = Paragraph::new(content)
        .block(content_block)
        .style(theme.panel_background())
        .alignment(Alignment::Center);

    f.render_widget(paragraph, chunks[1]);
}

/// Render content for a repository
fn render_repo_content(repo: &Repository, theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    let repo_name = format!("{}/{}@{}", repo.org, repo.repo, repo.branch);
    vec![
        Line::from(""),
        Line::from(Span::styled(repo_name, theme.success().bold())),
        Line::from(""),
        Line::from(Span::styled(
            "Repository content will be displayed here",
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
            Span::styled("- Show debug console", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl + p       ", theme.key_hint()),
            Span::styled("- Show Command Palette", theme.key_description()),
        ]),
        Line::from(vec![
            Span::styled("  q or Esc       ", theme.key_hint()),
            Span::styled("- Quit", theme.key_description()),
        ]),
    ]
}

/// Render content when no repositories are configured
fn render_no_repos_content(theme: &crate::theme::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "No Repositories",
            theme.text_secondary().bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press  p → a  to add your first repository",
            theme.muted(),
        )),
        Line::from(""),
    ]
}

/// Modern background-color style tabs widget
/// Uses background colors instead of borders - active tab has prominent color,
/// inactive tabs are subtle. Content frame matches selected tab's color.
struct ModernTabs<'a> {
    titles: Vec<String>,
    selected: usize,
    theme: &'a Theme,
}

impl<'a> ModernTabs<'a> {
    fn new(titles: Vec<String>, selected: usize, theme: &'a Theme) -> Self {
        Self {
            titles,
            selected,
            theme,
        }
    }
}

impl Widget for ModernTabs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < 10 {
            return;
        }

        // Modern style: background colors only, no borders on tabs
        // Selected:   ████████████ (prominent color)
        // Unselected: ▒▒▒▒▒▒▒▒▒▒▒▒ (subtle)

        let mut x = area.x + 1; // Start with a small margin

        // Colors
        let active_bg = self.theme.accent_primary;
        let active_fg = self.theme.bg_primary;
        let inactive_bg = self.theme.bg_tertiary;
        let inactive_fg = self.theme.text_muted;

        // Render each tab (just 1 row of content with background)
        for (i, title) in self.titles.iter().enumerate() {
            let is_selected = i == self.selected;
            let tab_width = title.len() as u16 + 4; // 2 chars padding on each side

            if x + tab_width > area.x + area.width {
                break; // Don't overflow
            }

            // Style based on selection
            let style = if is_selected {
                ratatui::style::Style::default()
                    .fg(active_fg)
                    .bg(active_bg)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                ratatui::style::Style::default()
                    .fg(inactive_fg)
                    .bg(inactive_bg)
            };

            // Render tab background and text
            let padded_title = format!("  {}  ", title);
            buf.set_string(x, area.y, &padded_title, style);

            x += tab_width + 1; // Gap between tabs
        }

        // Render "add repository" hint tab at the end
        let hint_text = " p → a ";
        let hint_width = hint_text.len() as u16;
        if x + hint_width <= area.x + area.width {
            let hint_style = ratatui::style::Style::default()
                .fg(self.theme.text_muted)
                .add_modifier(ratatui::style::Modifier::DIM);
            buf.set_string(x, area.y, hint_text, hint_style);
        }
    }
}
