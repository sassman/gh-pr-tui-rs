//! Debug Console View

use crate::actions::{Action, ContextAction, DebugConsoleAction, NavigationAction};
use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::keybindings::Keymap;
use crate::state::AppState;
use crate::state::DebugConsoleState;
use crate::view_models::debug_console_view_model::DebugConsoleViewModel;
use crate::views::View;
use gh_pr_lander_theme::Theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Debug console view - shows logs overlaid on main view
#[derive(Debug, Clone)]
pub struct DebugConsoleView;

impl DebugConsoleView {
    pub fn new() -> Self {
        Self
    }
}

impl View for DebugConsoleView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::DebugConsole
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        // Render debug console - this is a floating view so it renders on top
        render(&state.debug_console, &state.theme, &state.keymap, area, f);
    }

    fn capabilities(&self, state: &AppState) -> PanelCapabilities {
        // Debug console has its own capabilities
        state.debug_console.capabilities()
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
        let action = match nav {
            NavigationAction::Next => DebugConsoleAction::NavigateNext,
            NavigationAction::Previous => DebugConsoleAction::NavigatePrevious,
            NavigationAction::ToTop => DebugConsoleAction::NavigateToTop,
            NavigationAction::ToBottom => DebugConsoleAction::NavigateToBottom,
            // Debug console doesn't use horizontal navigation
            NavigationAction::Left | NavigationAction::Right => return None,
        };
        Some(Action::DebugConsole(action))
    }

    fn translate_context_action(
        &self,
        _action: ContextAction,
        _state: &AppState,
    ) -> Option<Action> {
        // Debug console is read-only log viewer, no context actions apply
        None
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::DebugConsole(_)
                | Action::ViewContext(_)
                | Action::Navigate(_)
                | Action::Global(_)
        )
    }
}

/// Render the debug console (Quake-style drop-down)
fn render(state: &DebugConsoleState, theme: &Theme, keymap: &Keymap, area: Rect, f: &mut Frame) {
    // Render dimmed overlay over the entire screen to create modal effect
    let overlay = Block::default().style(
        ratatui::style::Style::default()
            .bg(ratatui::style::Color::Black)
            .add_modifier(ratatui::style::Modifier::DIM),
    );
    f.render_widget(overlay, area);

    // Calculate console height based on percentage
    let console_height = (area.height * 70) / 100;
    let console_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: console_height.min(area.height),
    };

    // Clear the console area (removes the dim effect for the console itself)
    f.render_widget(Clear, console_area);

    // Create view model with pre-computed hints
    let view_model = DebugConsoleViewModel::new(state, keymap);

    // Build footer hint for bottom border using pre-computed hints from view model
    let footer_hint = Line::from(vec![
        Span::styled(
            format!(" {}", view_model.footer_hints.scroll),
            theme.key_hint().bold(),
        ),
        Span::styled(" scroll  ", theme.muted()),
        Span::styled(&view_model.footer_hints.top_bottom, theme.key_hint().bold()),
        Span::styled(" top/bottom  ", theme.muted()),
        Span::styled(&view_model.footer_hints.close, theme.key_hint().bold()),
        Span::styled(" close ", theme.muted()),
    ]);

    let block = Block::default()
        .title(view_model.title())
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title())
        .title_bottom(footer_hint)
        .title_alignment(Alignment::Center);

    // Calculate visible window
    let available_height = console_height.saturating_sub(2) as usize; // -2 for borders

    // Get visible lines and format them
    let visible_lines = view_model.visible_lines(available_height);
    let formatted_lines: Vec<Line> = visible_lines
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), theme.text())))
        .collect();

    let paragraph = Paragraph::new(formatted_lines)
        .block(block)
        .style(theme.panel_background());

    f.render_widget(paragraph, console_area);
}
