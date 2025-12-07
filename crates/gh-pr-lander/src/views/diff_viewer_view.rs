//! Diff Viewer View
//!
//! Renders the diff viewer panel for reviewing PR changes.

use crate::actions::{
    Action, AvailableAction, ContextAction, DiffViewerAction, NavigationAction, TextInputAction,
};
use crate::capabilities::PanelCapabilities;
use crate::command_id::CommandId;
use crate::state::AppState;
use crate::view_models::StatusBarViewModel;
use crate::views::status_bar::StatusBarWidget;
use crate::views::{View, ViewId};
use gh_diff_viewer::{DiffHighlighter, DiffViewer, FooterHint, ThemeProvider};
use ratatui::{prelude::*, widgets::*};

/// Diff viewer view - displays PR diff with syntax highlighting
#[derive(Debug, Clone)]
pub struct DiffViewerView;

impl DiffViewerView {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DiffViewerView {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme adapter to bridge gh-pr-lander-theme with gh-diff-viewer's ThemeProvider
struct LanderThemeAdapter<'a>(&'a gh_pr_lander_theme::Theme);

impl ThemeProvider for LanderThemeAdapter<'_> {
    fn addition_background(&self) -> Color {
        Color::Rgb(20, 40, 20) // Dark green tint
    }

    fn deletion_background(&self) -> Color {
        Color::Rgb(40, 20, 20) // Dark red tint
    }

    fn context_background(&self) -> Color {
        self.0.bg_panel
    }

    fn hunk_header_background(&self) -> Color {
        Color::Rgb(30, 30, 50) // Dark blue tint
    }

    fn hunk_header_foreground(&self) -> Color {
        self.0.accent_primary
    }

    fn line_number_foreground(&self) -> Color {
        self.0.text_muted
    }

    fn cursor_background(&self) -> Color {
        self.0.selected_bg
    }

    fn cursor_foreground(&self) -> Color {
        self.0.selected_fg
    }

    fn comment_indicator_foreground(&self) -> Color {
        self.0.accent_primary
    }

    fn expansion_marker_foreground(&self) -> Color {
        self.0.text_muted
    }

    fn expansion_marker_background(&self) -> Color {
        self.0.bg_panel
    }

    fn file_tree_border(&self) -> Color {
        self.0.text_muted
    }

    fn file_tree_selected_foreground(&self) -> Color {
        self.0.active_fg
    }

    fn file_tree_selected_background(&self) -> Color {
        self.0.selected_bg
    }

    fn file_tree_directory_foreground(&self) -> Color {
        self.0.accent_primary
    }

    fn hint_key_foreground(&self) -> Color {
        self.0.accent_primary
    }

    fn hint_text_foreground(&self) -> Color {
        self.0.text_muted
    }
}

impl View for DiffViewerView {
    fn view_id(&self) -> ViewId {
        ViewId::DiffViewer
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        // Split area to preserve status bar at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Diff viewer content
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        // Use Clear widget to completely clear the underlying content
        f.render_widget(Clear, chunks[0]);

        // Render background
        let background = Block::default().style(Style::default().bg(state.theme.bg_panel));
        f.render_widget(background, chunks[0]);

        // Check loading state
        if state.diff_viewer.is_loading() {
            let loading_msg = Paragraph::new("Loading diff...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Diff Viewer ")
                        .border_style(Style::default().fg(state.theme.accent_primary))
                        .style(Style::default().bg(state.theme.bg_panel)),
                )
                .style(
                    Style::default()
                        .fg(state.theme.text_muted)
                        .bg(state.theme.bg_panel),
                )
                .alignment(Alignment::Center);
            f.render_widget(loading_msg, chunks[0]);
        } else if let Some(error) = state.diff_viewer.error_message() {
            let error_msg = Paragraph::new(format!("Error: {}", error))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Diff Viewer - Error ")
                        .border_style(Style::default().fg(state.theme.status_error))
                        .style(Style::default().bg(state.theme.bg_panel)),
                )
                .style(
                    Style::default()
                        .fg(state.theme.status_error)
                        .bg(state.theme.bg_panel),
                )
                .alignment(Alignment::Center);
            f.render_widget(error_msg, chunks[0]);
        } else if let Some(ref inner_state) = state.diff_viewer.inner {
            // Create theme adapter
            let theme_adapter = LanderThemeAdapter(&state.theme);

            // Create a mutable copy of the highlighter for rendering
            let mut highlighter = DiffHighlighter::new();

            // Build footer hints based on current mode
            let hints = if inner_state.is_editing_comment() {
                vec![
                    FooterHint::new("Enter", "Submit"),
                    FooterHint::new("Esc", "Cancel"),
                ]
            } else if inner_state.show_review_popup {
                vec![
                    FooterHint::new("Enter", "Submit"),
                    FooterHint::new("←/→", "Select"),
                    FooterHint::new("Esc", "Cancel"),
                ]
            } else {
                vec![
                    FooterHint::new("c", "Comment"),
                    FooterHint::new("R", "Review"),
                    FooterHint::new("q", "Close"),
                ]
            };

            // Create the diff viewer widget with theme and hints
            let widget = DiffViewer::new(&mut highlighter, &theme_adapter)
                .with_footer_hints(hints);

            // We need to clone the inner state for rendering since render_with_state requires &mut
            let mut render_state = inner_state.clone();
            widget.render_with_state(chunks[0], f.buffer_mut(), &mut render_state);
        } else {
            // No diff loaded - show empty state
            let empty_msg = Paragraph::new("No diff loaded. Press 'd d' on a PR to view its diff.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Diff Viewer ")
                        .border_style(Style::default().fg(state.theme.accent_primary))
                        .style(Style::default().bg(state.theme.bg_panel)),
                )
                .style(
                    Style::default()
                        .fg(state.theme.text_muted)
                        .bg(state.theme.bg_panel),
                )
                .alignment(Alignment::Center);
            f.render_widget(empty_msg, chunks[0]);
        }

        // Render status bar
        let status_vm = StatusBarViewModel::from_state(state);
        f.render_widget(StatusBarWidget(&status_vm), chunks[1]);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        PanelCapabilities::SCROLL_VERTICAL
            | PanelCapabilities::SCROLL_HORIZONTAL
            | PanelCapabilities::VIM_SCROLL_BINDINGS
            | PanelCapabilities::VIM_NAVIGATION_BINDINGS
            | PanelCapabilities::ITEM_NAVIGATION
            | PanelCapabilities::TEXT_INPUT
            | PanelCapabilities::PANE_SWITCHING
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn translate_navigation(&self, nav: NavigationAction) -> Option<Action> {
        let action = match nav {
            NavigationAction::Next => DiffViewerAction::NavigateDown,
            NavigationAction::Previous => DiffViewerAction::NavigateUp,
            NavigationAction::Left => DiffViewerAction::NavigateLeft,
            NavigationAction::Right => DiffViewerAction::NavigateRight,
            NavigationAction::ToTop => DiffViewerAction::NavigateToTop,
            NavigationAction::ToBottom => DiffViewerAction::NavigateToBottom,
        };
        Some(Action::DiffViewer(action))
    }

    fn translate_text_input(&self, input: TextInputAction) -> Option<Action> {
        // All input is forwarded to the reducer which decides based on inner state mode
        // (normal navigation, comment editing, or review popup)
        match input {
            // All character keys go through KeyPress - reducer routes based on mode
            TextInputAction::Char(c) => Some(Action::DiffViewer(DiffViewerAction::KeyPress(c))),

            // Backspace - reducer checks if in comment mode
            TextInputAction::Backspace => Some(Action::DiffViewer(DiffViewerAction::Backspace)),

            // Escape - context-aware (cancel comment, hide popup, or focus tree)
            TextInputAction::Escape => {
                Some(Action::DiffViewer(DiffViewerAction::EscapeOrFocusTree))
            }

            // Confirm/Enter - context-aware (commit comment, submit review, or toggle)
            TextInputAction::Confirm => Some(Action::DiffViewer(DiffViewerAction::Confirm)),

            // Clear line - not used in diff viewer
            TextInputAction::ClearLine => None,
        }
    }

    fn translate_context_action(&self, action: ContextAction, _state: &AppState) -> Option<Action> {
        match action {
            // Confirm toggles expand/collapse in file tree or confirms comment
            ContextAction::Confirm => Some(Action::DiffViewer(DiffViewerAction::Toggle)),
            // ToggleSelect enters visual mode for line selection
            ContextAction::ToggleSelect => {
                Some(Action::DiffViewer(DiffViewerAction::EnterVisualMode))
            }
            _ => None,
        }
    }

    fn accepts_action(&self, action: &Action) -> bool {
        matches!(
            action,
            Action::DiffViewer(_)
                | Action::ViewContext(_)
                | Action::Navigate(_)
                | Action::TextInput(_) // | Action::Global(_)
        )
    }

    fn available_actions(&self, state: &AppState) -> Vec<AvailableAction> {
        // Show different actions based on current mode
        if let Some(ref inner) = state.diff_viewer.inner {
            if inner.is_editing_comment() {
                // Comment editing mode
                return vec![
                    AvailableAction::primary(CommandId::Confirm, "Submit"),
                    AvailableAction::navigation(CommandId::GlobalClose, "Cancel"),
                ];
            }
            if inner.show_review_popup {
                // Review popup mode
                return vec![
                    AvailableAction::primary(CommandId::Confirm, "Submit Review"),
                    AvailableAction::navigation(CommandId::NavigateLeft, "Prev"),
                    AvailableAction::navigation(CommandId::NavigateRight, "Next"),
                    AvailableAction::navigation(CommandId::GlobalClose, "Cancel"),
                ];
            }
        }

        // Normal mode
        vec![
            AvailableAction::primary(CommandId::Confirm, "Toggle/Select"),
            AvailableAction::primary(CommandId::DiffViewerAddComment, "Comment"),
            AvailableAction::primary(CommandId::DiffViewerShowReviewPopup, "Review"),
            AvailableAction::navigation(CommandId::NavigateNext, "Down"),
            AvailableAction::navigation(CommandId::DiffViewerSwitchPane, "Switch Pane"),
            AvailableAction::navigation(CommandId::GlobalClose, "Close"),
            AvailableAction::navigation(CommandId::KeyBindingsToggleView, "Help"),
        ]
    }
}
