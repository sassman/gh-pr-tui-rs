//! Confirmation Popup View
//!
//! A reusable floating popup for confirming PR operations with editable message.
//! Used for approve, comment, request changes, and close actions.

use crate::actions::{
    Action, ConfirmationPopupAction, ContextAction, NavigationAction, TextInputAction,
};
use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::view_models::ConfirmationPopupViewModel;
use crate::views::{View, ViewId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Confirmation popup view - floating form for confirming PR actions
#[derive(Debug, Clone)]
pub struct ConfirmationPopupView;

impl ConfirmationPopupView {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfirmationPopupView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for ConfirmationPopupView {
    fn view_id(&self) -> ViewId {
        ViewId::ConfirmationPopup
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        if let Some(ref popup_state) = state.confirmation_popup {
            let vm = ConfirmationPopupViewModel::from_state(popup_state, &state.theme);
            render_popup(f, &vm, &state.theme, area);
        }
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Confirmation popup accepts text input
        PanelCapabilities::TEXT_INPUT
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn translate_navigation(&self, _nav: NavigationAction) -> Option<Action> {
        // Popup doesn't handle navigation
        None
    }

    fn translate_text_input(&self, input: TextInputAction) -> Option<Action> {
        let action = match input {
            TextInputAction::Char(c) => ConfirmationPopupAction::Char(c),
            TextInputAction::Backspace => ConfirmationPopupAction::Backspace,
            TextInputAction::ClearLine => ConfirmationPopupAction::ClearInput,
            TextInputAction::Escape => ConfirmationPopupAction::Cancel,
            TextInputAction::Confirm => ConfirmationPopupAction::Confirm,
        };
        Some(Action::ConfirmationPopup(action))
    }

    fn translate_context_action(&self, action: ContextAction, _state: &AppState) -> Option<Action> {
        match action {
            // Confirm submits the popup
            ContextAction::Confirm => {
                Some(Action::ConfirmationPopup(ConfirmationPopupAction::Confirm))
            }
            // Selection actions don't apply to a popup
            _ => None,
        }
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::ConfirmationPopup(_)
                | Action::ViewContext(_)
                | Action::Navigate(_)
                | Action::TextInput(_)
                | Action::Global(_)
        )
    }
}

/// Render the confirmation popup as a centered floating window
fn render_popup(
    f: &mut Frame,
    vm: &ConfirmationPopupViewModel,
    theme: &gh_pr_lander_theme::Theme,
    area: Rect,
) {
    // Render dimmed overlay over the entire screen to create modal effect
    let overlay = Block::default().style(
        Style::default()
            .bg(ratatui::style::Color::Black)
            .add_modifier(Modifier::DIM),
    );
    f.render_widget(overlay, area);

    // Calculate centered area (60% width, fixed height)
    let popup_width = (area.width * 60 / 100).clamp(50, 70);
    let popup_height = 10; // Fixed height for the popup
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

    // Build footer hint for bottom border
    let footer_hint = Line::from(vec![
        Span::styled(" ", theme.muted()),
        Span::styled(&vm.footer_hints.confirm, theme.key_hint().bold()),
        Span::styled(" confirm  ", theme.muted()),
        Span::styled(&vm.footer_hints.cancel, theme.key_hint().bold()),
        Span::styled(" cancel ", theme.muted()),
    ]);

    // Render border with title at top and hints at bottom
    let title = format!(" {} ", vm.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(theme.panel_title().add_modifier(Modifier::BOLD))
        .title_bottom(footer_hint)
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_style(
            Style::default()
                .fg(vm.colors.border_fg)
                .add_modifier(Modifier::BOLD),
        )
        .style(theme.panel_background());

    f.render_widget(block, popup_area);

    // Calculate inner area
    let inner = popup_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Target info (e.g., "Approving: PR #123")
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Instructions
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // Input field
            Constraint::Length(1), // Validation hint
            Constraint::Min(0),    // Remaining
        ])
        .split(inner);

    // Target info line
    let target_line = Line::from(Span::styled(
        &vm.target_line,
        Style::default()
            .fg(vm.colors.target_fg)
            .add_modifier(Modifier::BOLD),
    ));
    f.render_widget(Paragraph::new(target_line), chunks[0]);

    // Instructions
    let instructions = Line::from(Span::styled(
        &vm.instructions,
        Style::default().fg(vm.colors.instructions_fg),
    ));
    f.render_widget(Paragraph::new(instructions), chunks[2]);

    // Input field
    render_input_field(f, chunks[4], vm, theme);

    // Validation hint (if present)
    if let Some(ref hint) = vm.validation_hint {
        let hint_line = Line::from(Span::styled(
            hint,
            Style::default()
                .fg(vm.colors.error_fg)
                .add_modifier(Modifier::ITALIC),
        ));
        f.render_widget(Paragraph::new(hint_line), chunks[5]);
    }
}

/// Render the message input field
fn render_input_field(
    f: &mut Frame,
    area: Rect,
    vm: &ConfirmationPopupViewModel,
    theme: &gh_pr_lander_theme::Theme,
) {
    let label = &vm.input_label;
    let value = &vm.input_value;

    // Build the input line with cursor
    let line = Line::from(vec![
        Span::styled(
            format!("{} ", label),
            Style::default()
                .fg(vm.colors.input_label_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            value,
            Style::default()
                .fg(vm.colors.input_fg)
                .bg(vm.colors.input_bg),
        ),
        // Cursor indicator
        Span::styled("▌", theme.accent()),
    ]);

    f.render_widget(Paragraph::new(line), area);
}
