//! Add Repository View
//!
//! A floating form for adding a new repository to track.
//! Supports both URL parsing and manual field entry.

use crate::actions::{
    Action, AddRepositoryAction, ContextAction, NavigationAction, TextInputAction,
};
use crate::capabilities::PanelCapabilities;
use crate::state::{AddRepoField, AddRepoFormState, AppState};
use crate::views::View;
use gh_pr_lander_theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Add repository view - floating form for adding new repositories
#[derive(Debug, Clone)]
pub struct AddRepositoryView;

impl AddRepositoryView {
    pub fn new() -> Self {
        Self
    }
}

impl View for AddRepositoryView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::AddRepository
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        render(&state.add_repo_form, &state.theme, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Add repository form accepts text input
        PanelCapabilities::TEXT_INPUT
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
        let action = match nav {
            NavigationAction::Next => AddRepositoryAction::NextField,
            NavigationAction::Previous => AddRepositoryAction::PrevField,
            // Form doesn't use horizontal or jump navigation
            NavigationAction::Left
            | NavigationAction::Right
            | NavigationAction::ToTop
            | NavigationAction::ToBottom => return None,
        };
        Some(Action::AddRepository(action))
    }

    fn translate_text_input(&self, input: TextInputAction) -> Option<Action> {
        let action = match input {
            TextInputAction::Char(c) => AddRepositoryAction::Char(c),
            TextInputAction::Backspace => AddRepositoryAction::Backspace,
            TextInputAction::ClearLine => AddRepositoryAction::ClearField,
            TextInputAction::Escape => AddRepositoryAction::Close,
            TextInputAction::Confirm => AddRepositoryAction::Confirm,
        };
        Some(Action::AddRepository(action))
    }

    fn translate_context_action(&self, action: ContextAction, _state: &AppState) -> Option<Action> {
        match action {
            // Confirm submits the form
            ContextAction::Confirm => Some(Action::AddRepository(AddRepositoryAction::Confirm)),
            // Selection actions don't apply to a form
            _ => None,
        }
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::AddRepository(_)
                | Action::ViewContext(_)
                | Action::Navigate(_)
                | Action::TextInput(_)
                | Action::Global(_)
        )
    }
}

/// Render the add repository popup as a centered floating window
fn render(form: &AddRepoFormState, theme: &Theme, area: Rect, f: &mut Frame) {
    // Render dimmed overlay over the entire screen to create modal effect
    let overlay = Block::default().style(
        ratatui::style::Style::default()
            .bg(ratatui::style::Color::Black)
            .add_modifier(Modifier::DIM),
    );
    f.render_widget(overlay, area);

    // Calculate centered area (65% width, fixed height)
    let popup_width = (area.width * 65 / 100).clamp(55, 80);
    let popup_height = 16; // Fixed height for the form
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
        Span::styled(" Tab", theme.key_hint().bold()),
        Span::styled("/", theme.muted()),
        Span::styled("S-Tab", theme.key_hint().bold()),
        Span::styled(" navigate  ", theme.muted()),
        Span::styled("Enter", theme.key_hint().bold()),
        Span::styled(" add  ", theme.muted()),
        Span::styled("Esc", theme.key_hint().bold()),
        Span::styled(" cancel ", theme.muted()),
    ]);

    // Render border with title at top and hints at bottom
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add New Repository ")
        .title_style(theme.panel_title().add_modifier(Modifier::BOLD))
        .title_bottom(footer_hint)
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_style(theme.panel_border().add_modifier(Modifier::BOLD))
        .style(theme.panel_background());

    f.render_widget(block, popup_area);

    // Calculate inner area
    let inner = popup_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split into sections (footer is now in the border, so we don't need a row for it)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Instructions
            Constraint::Length(1), // Spacing
            Constraint::Length(1), // URL field
            Constraint::Length(1), // Spacing/separator
            Constraint::Length(1), // Org field
            Constraint::Length(1), // Repo field
            Constraint::Length(1), // Branch field
            Constraint::Min(0),    // Remaining
        ])
        .split(inner);

    // Instructions
    let instructions = Line::from(vec![Span::styled(
        "Enter GitHub URL or fill in the fields manually:",
        theme.text_secondary(),
    )]);
    f.render_widget(Paragraph::new(instructions), chunks[0]);

    // URL field (with cursor indicator)
    render_field(
        f,
        chunks[2],
        "GitHub URL",
        &form.url,
        form.focused_field == AddRepoField::Url,
        theme,
        Some("e.g. https://github.com/org/repo.git"),
    );

    // Separator line
    let separator = Line::from(vec![Span::styled(
        "─── or fill manually ───",
        theme.muted(),
    )]);
    f.render_widget(
        Paragraph::new(separator).alignment(Alignment::Center),
        chunks[3],
    );

    // Org field
    render_field(
        f,
        chunks[4],
        "Organization",
        &form.org,
        form.focused_field == AddRepoField::Org,
        theme,
        None,
    );

    // Repo field
    render_field(
        f,
        chunks[5],
        "Repository",
        &form.repo,
        form.focused_field == AddRepoField::Repo,
        theme,
        None,
    );

    // Branch field
    render_field(
        f,
        chunks[6],
        "Branch",
        &form.branch,
        form.focused_field == AddRepoField::Branch,
        theme,
        Some("default: main"),
    );
}

/// Render a single form field
fn render_field(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    theme: &Theme,
    placeholder: Option<&str>,
) {
    // Calculate label width (14 chars for alignment)
    let label_width = 14;

    let indicator = if focused { "> " } else { "  " };

    let label_style = if focused {
        theme.text().add_modifier(Modifier::BOLD)
    } else {
        theme.text()
    };

    // Determine what to show in the value area
    let (display_value, value_style, is_placeholder) = if value.is_empty() {
        if let Some(ph) = placeholder {
            // Placeholder: dimmed, italic, muted color
            (ph, theme.muted().italic().add_modifier(Modifier::DIM), true)
        } else {
            ("", theme.text(), false)
        }
    } else {
        (value, theme.text(), false)
    };

    // Build the line - cursor position depends on whether showing placeholder
    let line = if focused && is_placeholder {
        // When focused with placeholder: cursor at beginning, then placeholder text
        Line::from(vec![
            Span::styled(indicator, theme.accent().bold()),
            Span::styled(
                format!("{:width$}", format!("{}:", label), width = label_width),
                label_style,
            ),
            Span::styled("▌", theme.accent()),
            Span::styled(display_value, value_style),
        ])
    } else {
        // Normal case: text (or empty), then cursor if focused
        Line::from(vec![
            Span::styled(indicator, theme.accent().bold()),
            Span::styled(
                format!("{:width$}", format!("{}:", label), width = label_width),
                label_style,
            ),
            Span::styled(
                display_value,
                if focused {
                    // Use active_fg for high contrast text when focused
                    ratatui::style::Style::default()
                        .fg(theme.active_fg)
                        .bg(theme.selection_bg())
                } else {
                    value_style
                },
            ),
            // Show cursor when focused
            if focused {
                Span::styled("▌", theme.accent())
            } else {
                Span::raw("")
            },
        ])
    };

    f.render_widget(Paragraph::new(line), area);
}
