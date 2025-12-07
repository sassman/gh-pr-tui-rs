//! Key Bindings Help Panel View
//!
//! Displays all available keybindings grouped by category.

use crate::actions::{Action, ContextAction, KeyBindingsAction, NavigationAction};
use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::view_models::KeyBindingsPanelViewModel;
use crate::views::View;
use gh_pr_lander_theme::Theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Key bindings help panel view
#[derive(Debug, Clone)]
pub struct KeyBindingsView;

impl KeyBindingsView {
    pub fn new() -> Self {
        Self
    }
}

impl View for KeyBindingsView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::KeyBindings
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        render(state, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Key bindings panel supports vertical scrolling with vim bindings
        PanelCapabilities::SCROLL_VERTICAL | PanelCapabilities::VIM_NAVIGATION_BINDINGS
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
        let action = match nav {
            NavigationAction::Next => KeyBindingsAction::NavigateNext,
            NavigationAction::Previous => KeyBindingsAction::NavigatePrevious,
            NavigationAction::ToTop => KeyBindingsAction::NavigateToTop,
            NavigationAction::ToBottom => KeyBindingsAction::NavigateToBottom,
            // Key bindings panel doesn't use horizontal navigation
            NavigationAction::Left | NavigationAction::Right => return None,
        };
        Some(Action::KeyBindings(action))
    }

    fn translate_context_action(
        &self,
        _action: ContextAction,
        _state: &AppState,
    ) -> Option<Action> {
        // Key bindings panel is read-only, no context actions apply
        None
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::KeyBindings(_)
                | Action::ViewContext(_)
                | Action::Navigate(_)
                | Action::Global(_)
        )
    }
}

/// Render the key bindings panel
fn render(state: &AppState, area: Rect, f: &mut Frame) {
    let theme = &state.theme;

    // Calculate panel size (60% width, 90% height, centered)
    let panel_width = (area.width * 60) / 100;
    let panel_height = (area.height * 90) / 100;

    let panel_x = area.x + (area.width - panel_width) / 2;
    let panel_y = area.y + (area.height - panel_height) / 2;

    let panel_area = Rect {
        x: panel_x,
        y: panel_y,
        width: panel_width,
        height: panel_height,
    };

    // Render dimmed overlay over the entire screen
    let overlay = Block::default().style(
        ratatui::style::Style::default()
            .bg(ratatui::style::Color::Black)
            .add_modifier(ratatui::style::Modifier::DIM),
    );
    f.render_widget(overlay, area);

    // Clear the panel area
    f.render_widget(Clear, panel_area);

    // Build view model
    let vm = KeyBindingsPanelViewModel::from_state(state);

    // Build footer hint
    let footer_hint = Line::from(vec![
        Span::styled(
            format!(" {}", vm.footer_hints.scroll),
            theme.key_hint().bold(),
        ),
        Span::styled(" scroll  ", theme.muted()),
        Span::styled(&vm.footer_hints.close, theme.key_hint().bold()),
        Span::styled(" close ", theme.muted()),
    ]);

    let block = Block::default()
        .title(vm.title.clone())
        .borders(Borders::ALL)
        .border_style(theme.panel_border())
        .title_style(theme.panel_title())
        .title_alignment(Alignment::Center)
        .title_bottom(footer_hint);

    // Calculate inner area for content
    let inner_area = block.inner(panel_area);

    // Build content lines
    let lines = build_content_lines(&vm, theme, inner_area.width as usize);

    // Apply scroll offset
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(vm.scroll_offset)
        .take(inner_area.height as usize)
        .collect();

    let paragraph = Paragraph::new(visible_lines)
        .block(block)
        .style(theme.panel_background());

    f.render_widget(paragraph, panel_area);
}

/// Left padding for content
const LEFT_PADDING: &str = "  ";

/// Build all content lines for the panel
fn build_content_lines<'a>(
    vm: &KeyBindingsPanelViewModel,
    theme: &Theme,
    _width: usize,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();

    for section in &vm.sections {
        // Category header with padding
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(section.category.clone(), theme.section_header()),
        ]));

        // Separator line with padding
        let separator = "─".repeat(section.category.len());
        lines.push(Line::from(vec![
            Span::raw(LEFT_PADDING),
            Span::styled(separator, theme.muted()),
        ]));

        // Binding rows with padding
        for binding in &section.bindings {
            let padding_span = Span::raw(LEFT_PADDING);
            let key_span = Span::styled(format!("{:<16}", binding.keys), theme.key_hint());
            let desc_span = Span::styled(binding.description.clone(), theme.key_description());

            lines.push(Line::from(vec![padding_span, key_span, desc_span]));
        }

        // Empty line after section
        lines.push(Line::default());
    }

    lines
}
