//! Status Bar Widget
//!
//! Renders the status bar at the bottom of the screen.
//! Format: `[timestamp] emoji message                     [source]`

use crate::view_models::StatusBarViewModel;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

/// Widget for rendering the status bar
pub struct StatusBarWidget<'a>(pub &'a StatusBarViewModel);

impl Widget for StatusBarWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vm = self.0;

        if area.height < 1 {
            return;
        }

        // Fill entire row with background
        for x in area.x..area.x + area.width {
            buf[(x, area.y)].set_bg(vm.bg_color).set_char(' ');
        }

        if !vm.has_message {
            return;
        }

        let mut x = area.x + 1; // 1 char padding

        // Timestamp in brackets (if present)
        if !vm.timestamp.is_empty() {
            let ts_str = format!("[{}] ", vm.timestamp);
            buf.set_string(x, area.y, &ts_str, vm.metadata_style);
            x += ts_str.len() as u16;
        }

        // Emoji (estimate 2 chars width for most emoji)
        let emoji_str = format!("{} ", vm.emoji);
        buf.set_string(x, area.y, &emoji_str, vm.message_style);
        x += 3; // emoji + space (emoji typically renders as 2 cells)

        // Calculate space for source on right
        let source_width = if !vm.source.is_empty() {
            vm.source.len() + 3 // "[source] "
        } else {
            0
        };

        // Message (truncate if needed)
        let available_width = area
            .width
            .saturating_sub(x - area.x + source_width as u16 + 2);

        if vm.message.len() > available_width as usize {
            // Truncate with ellipsis
            let truncate_at = available_width.saturating_sub(1) as usize;
            let truncated: String = vm.message.chars().take(truncate_at).collect();
            let display = format!("{}…", truncated);
            buf.set_string(x, area.y, &display, vm.message_style);
        } else {
            buf.set_string(x, area.y, &vm.message, vm.message_style);
        }

        // Source on the right side
        if !vm.source.is_empty() {
            let source_str = format!("[{}]", vm.source);
            let source_x = area.x + area.width - source_str.len() as u16 - 1;
            buf.set_string(source_x, area.y, &source_str, vm.metadata_style);
        }
    }
}
