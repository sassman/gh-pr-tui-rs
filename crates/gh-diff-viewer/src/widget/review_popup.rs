//! Review submission popup widget.

use crate::model::ReviewEvent;
use crate::traits::ThemeProvider;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Widget};

/// Widget for the review submission popup.
///
/// Shows 4 buttons: Approve, Request Changes, Comment, Cancel
pub struct ReviewPopupWidget<'a, T: ThemeProvider> {
    /// Number of pending comments.
    comment_count: usize,
    /// Currently selected review event.
    selected: ReviewEvent,
    /// Theme provider.
    #[allow(dead_code)]
    theme: &'a T,
}

impl<'a, T: ThemeProvider> ReviewPopupWidget<'a, T> {
    /// Create a new review popup widget.
    pub fn new(comment_count: usize, selected: ReviewEvent, theme: &'a T) -> Self {
        Self {
            comment_count,
            selected,
            theme,
        }
    }
}

impl<T: ThemeProvider> Widget for ReviewPopupWidget<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate popup dimensions
        let popup_width = 50.min(area.width.saturating_sub(4));
        let popup_height = 7.min(area.height.saturating_sub(4));

        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the area behind the popup
        Clear.render(popup_area, buf);

        // Draw popup border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title(" Submit Review ");

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Render comment count
        let comment_text = if self.comment_count == 1 {
            "1 pending comment".to_string()
        } else {
            format!("{} pending comments", self.comment_count)
        };
        buf.set_string(
            inner.x,
            inner.y,
            &comment_text,
            Style::default().fg(Color::DarkGray),
        );

        // Render buttons
        let buttons = [
            ("Approve", ReviewEvent::Approve, Color::Green),
            ("Request Changes", ReviewEvent::RequestChanges, Color::Red),
            ("Comment", ReviewEvent::Comment, Color::Yellow),
        ];

        let button_y = inner.y + 2;
        let mut button_x = inner.x + 2;

        for (label, event, color) in buttons {
            let is_selected = self.selected == event;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            let text = if is_selected {
                format!(" [{}] ", label)
            } else {
                format!("  {}  ", label)
            };

            buf.set_string(button_x, button_y, &text, style);
            button_x += text.len() as u16 + 1;
        }

        // Render cancel hint
        let cancel_hint = "Esc: Cancel | Enter: Submit";
        let hint_x = inner.x + (inner.width.saturating_sub(cancel_hint.len() as u16)) / 2;
        buf.set_string(
            hint_x,
            inner.y + inner.height - 1,
            cancel_hint,
            Style::default().fg(Color::DarkGray),
        );

        // Render navigation hint
        let nav_hint = "←/→: Select";
        buf.set_string(
            inner.x,
            inner.y + inner.height - 1,
            nav_hint,
            Style::default().fg(Color::DarkGray),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::DefaultTheme;

    #[test]
    fn test_review_popup_widget_creation() {
        let theme = DefaultTheme;
        let _widget = ReviewPopupWidget::new(3, ReviewEvent::Approve, &theme);
    }
}
