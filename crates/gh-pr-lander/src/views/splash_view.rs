use crate::actions::Action;
use crate::capabilities::PanelCapabilities;
use crate::state::AppState;
use crate::views::View;
use figlet_rs::FIGfont;
use gh_pr_lander_theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

/// Splash screen view - shown during bootstrap
#[derive(Debug, Clone)]
pub struct SplashView;

impl SplashView {
    pub fn new() -> Self {
        Self
    }
}

impl View for SplashView {
    fn view_id(&self) -> crate::views::ViewId {
        crate::views::ViewId::Splash
    }

    fn render(&self, state: &AppState, area: Rect, f: &mut Frame) {
        render_splash(&state.splash, &state.theme, area, f);
    }

    fn capabilities(&self, _state: &AppState) -> PanelCapabilities {
        // Splash screen has no interactive capabilities
        PanelCapabilities::empty()
    }

    fn clone_box(&self) -> Box<dyn View> {
        Box::new(self.clone())
    }

    fn accepts_action(&self, action: &Action) -> bool {
        // Splash screen only accepts global actions (like Quit)
        matches!(action, Action::Global(_))
    }
}

/// Render the splash screen with snake loading animation
fn render_splash(state: &crate::state::SplashState, theme: &Theme, area: Rect, f: &mut Frame) {
    // Full screen background
    let background_block = Block::default().style(theme.panel_background());
    f.render_widget(background_block, area);

    // Generate FIGlet title
    let title_lines = generate_figlet_title(theme);
    let title_height = title_lines.len() as u16;

    // Title at the top
    let title_area = Rect {
        x: area.x,
        y: area.y + 2,
        width: area.width,
        height: title_height + 1,
    };

    let title = Paragraph::new(title_lines).alignment(Alignment::Center);
    f.render_widget(title, title_area);

    // Center the snake animation
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(8), // Height for snake + loading text
            Constraint::Percentage(40),
        ])
        .split(area);

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(12), // Width for 5x5 grid (each cell is ~2 chars wide)
            Constraint::Min(0),
        ])
        .split(vertical_chunks[1]);

    let center_area = horizontal_chunks[1];

    // Generate snake animation pattern (5x5 grid)
    let snake_lines = generate_snake_animation(state.animation_frame, theme);

    // Add loading text
    let mut lines = snake_lines;
    lines.push(Line::from(""));
    lines.push(
        Line::from(Span::styled("Loading...", theme.text().dim())).alignment(Alignment::Center),
    );

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(theme.panel_background());

    f.render_widget(paragraph, center_area);
}

/// Generate snake animation for the current frame
/// The snake chases its tail in a 5x5 grid pattern
fn generate_snake_animation(frame: usize, theme: &Theme) -> Vec<Line<'static>> {
    // Define the snake path (positions in reading order: row*5 + col)
    // Snake moves around the perimeter clockwise, then spirals inward
    let path = vec![
        0, 1, 2, 3, 4, // Top row →
        9, 14, 19, 24, // Right edge ↓
        23, 22, 21, 20, // Bottom row ←
        15, 10, 5, // Left edge ↑
        6, 7, 8, // Inner top row →
        13, 18, // Inner right edge ↓
        17, 16, // Inner bottom row ←
        11, // Inner left ↑
        12, // Center
    ];

    // Calculate which positions are "lit" based on the frame
    // Show up to 5 positions: the head and up to 4 trailing segments
    // Head is at 'frame', tail segments are at frame-1, frame-2, frame-3, frame-4
    let snake_length = 5;
    let mut lit_positions = [false; 25];

    for i in 0..snake_length {
        // Only show segments that actually exist (don't wrap around at the start)
        if frame >= i {
            let pos_index = frame - i;
            let grid_pos = path[pos_index % path.len()];
            lit_positions[grid_pos] = true;
        }
    }

    // Build the 5x5 grid
    let mut lines = Vec::new();
    for row in 0..5 {
        let mut spans = Vec::new();
        for col in 0..5 {
            let pos = row * 5 + col;
            let symbol = if lit_positions[pos] { "■" } else { "□" };
            let style = if lit_positions[pos] {
                theme.text().cyan().bold()
            } else {
                theme.muted().dim()
            };
            spans.push(Span::styled(format!("{} ", symbol), style));
        }
        lines.push(Line::from(spans));
    }

    lines
}

/// Embedded small FIGlet font
const SMALL_FONT: &str = include_str!("../../resources/small.flf");

/// Generate FIGlet title with merge icon
fn generate_figlet_title(theme: &Theme) -> Vec<Line<'static>> {
    // Merge icon (5 lines to match the FIGlet small font height)
    let icon = [
        "  ◉━━━◉  ",
        "   ╲ ╱   ",
        "    ◉    ",
        "    ┃    ",
        "    ◉    ",
    ];

    // Generate FIGlet text using the embedded "small" font
    let figlet_lines: Vec<String> = if let Ok(font) = FIGfont::from_content(SMALL_FONT) {
        if let Some(figure) = font.convert("GitHub PR Lander") {
            figure.to_string().lines().map(String::from).collect()
        } else {
            vec!["GitHub PR Lander".to_string()]
        }
    } else {
        vec!["GitHub PR Lander".to_string()]
    };

    // Combine icon and FIGlet text side by side
    let max_lines = icon.len().max(figlet_lines.len());
    let mut result = Vec::new();

    for i in 0..max_lines {
        let icon_part = icon.get(i).unwrap_or(&"         ");
        let figlet_part = figlet_lines.get(i).map(|s| s.as_str()).unwrap_or("");

        let line = Line::from(vec![
            Span::styled(icon_part.to_string(), theme.text().cyan().bold()),
            Span::styled("   ", theme.text()), // spacing
            Span::styled(figlet_part.to_string(), theme.panel_title().bold()),
        ]);
        result.push(line);
    }

    result
}
