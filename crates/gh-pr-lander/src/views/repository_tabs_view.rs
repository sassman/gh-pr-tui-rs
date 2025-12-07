use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::view_models::RepositoryTabsViewModel;

/// Left separator - lower right triangle (creates "/" slope into tab)
const LEFT_SEP: &str = "◢";
/// Right separator - lower left triangle (creates "\" slope out of tab)
const RIGHT_SEP: &str = "◣";

/// Widget wrapper for rendering repository tabs from view model
pub struct RepositoryTabsWidget<'a>(pub &'a RepositoryTabsViewModel);

impl Widget for RepositoryTabsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < 10 {
            return;
        }

        let vm = self.0;

        // Fill the entire row with the line background color first
        for x in area.x..area.x + area.width {
            buf[(x, area.y)].set_bg(vm.line_bg);
        }

        let mut x = area.x;

        // Render help hint on the far left
        buf.set_string(x, area.y, &vm.help_hint.text, vm.help_hint.style);
        x += vm.help_hint.width;

        // Render each tab with powerline separators
        for tab in &vm.tabs {
            if x + tab.width > area.x + area.width {
                break; // Don't overflow
            }

            // Left powerline separator
            buf.set_string(x, area.y, LEFT_SEP, tab.left_sep_style);
            x += 1;

            // Tab content with padding
            let padded_text = format!("  {}  ", tab.display_text);
            buf.set_string(x, area.y, &padded_text, tab.style);
            x += padded_text.chars().count() as u16;

            // Right powerline separator
            buf.set_string(x, area.y, RIGHT_SEP, tab.right_sep_style);
            x += 1;
        }

        // Render add repo hint at the end
        if x + vm.hint.width <= area.x + area.width {
            buf.set_string(x + 1, area.y, &vm.hint.text, vm.hint.style);
        }
    }
}
