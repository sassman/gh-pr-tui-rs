use octocrab::{Octocrab, params};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::event::KeyModifiers,
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    style::palette::tailwind,
};
use std::{error::Error, io};
use tokio::runtime::Runtime;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

struct App {
    state: TableState,
    items: Vec<Vec<String>>,
    repo: Repo,
    filter: PrFilter,
    selected_items: Vec<usize>,
    colors: TableColors,
}

struct Repo {
    org: String,
    repo: String,
    branch: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PrFilter {
    title: String,
}

impl Repo {
    fn new(org: &str, repo: &str, branch: &str) -> Repo {
        Repo {
            org: org.to_string(),
            repo: repo.to_string(),
            branch: branch.to_string(),
        }
    }
}

impl App {
    fn new() -> App {
        App {
            state: TableState::default(),
            items: Vec::new(),
            repo: Repo::new("cargo-generate", "cargo-generate", "main"),
            filter: PrFilter {
                title: "chore".to_string(),
            },
            selected_items: Vec::new(),
            colors: TableColors::new(&PALETTES[0]),
        }
    }

    fn fetch_data(&mut self) {
        let rt = Runtime::new().unwrap();
        let github_data = rt
            .block_on(fetch_github_data(&self.repo, &self.filter))
            .unwrap();
        self.items = github_data;
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn select(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if self.selected_items.contains(&i) {
            self.selected_items.retain(|&x| x != i);
        } else {
            self.selected_items.push(i);
        }
    }
}

async fn fetch_github_data(
    repo: &Repo,
    filter: &PrFilter,
) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let octocrab = Octocrab::builder().build()?;

    // Fetch some repos from the Rust organization as an example
    let page = octocrab
        .pulls(&repo.org, &repo.repo)
        .list()
        .state(params::State::Open)
        .head(&repo.branch)
        .sort(params::pulls::Sort::Updated)
        .direction(params::Direction::Ascending)
        .per_page(100)
        .send()
        .await?;

    let mut items = Vec::new();

    for pr in page.items.into_iter().filter(|pr| {
        pr.title
            .as_ref()
            .unwrap_or(&"".to_string())
            .contains(&filter.title)
    }) {
        let mergeable_state = if pr.mergeable_state.is_none() {
            let pr_no = pr.number;
            let pr_details = octocrab.pulls(&repo.org, &repo.repo).get(pr_no).await.ok();
            if let Some(pr_details) = pr_details {
                pr_details
                    .mergeable_state
                    .or(Some(octocrab::models::pulls::MergeableState::Unknown))
            } else {
                Some(octocrab::models::pulls::MergeableState::Unknown)
            }
        } else {
            Some(octocrab::models::pulls::MergeableState::Unknown)
        };

        let row = vec![
            pr.number.to_string(),
            pr.title.unwrap_or_default(),
            pr.comments.unwrap_or_default().to_string(),
            pr.mergeable_state
                .or(mergeable_state)
                .map(|merge_state| match merge_state {
                    octocrab::models::pulls::MergeableState::Behind => "n",
                    octocrab::models::pulls::MergeableState::Blocked => "n",
                    octocrab::models::pulls::MergeableState::Clean => "y",
                    octocrab::models::pulls::MergeableState::Dirty => "n",
                    octocrab::models::pulls::MergeableState::Draft => "n",
                    octocrab::models::pulls::MergeableState::HasHooks => "n",
                    octocrab::models::pulls::MergeableState::Unknown => "na",
                    octocrab::models::pulls::MergeableState::Unstable => "n",
                    _ => todo!(),
                })
                .unwrap()
                .to_string(),
        ];
        items.push(row);
    }

    Ok(items)
}

fn main() -> Result<(), Box<dyn Error>> {
    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    app.fetch_data();
    app.state.select(Some(0));

    // Main loop
    loop {
        terminal.draw(|f| {
            let size = f.area();
            let block = Block::default()
                .title(format!(
                    "GitHub PRs: {}/{}@{}",
                    &app.repo.org, &app.repo.repo, &app.repo.branch
                ))
                .borders(Borders::ALL);

            let header_style = Style::default()
                .fg(app.colors.header_fg)
                .bg(app.colors.header_bg);

            let header_cells = ["#PR", "Description", "#Comments", "Mergable"]
                .iter()
                .map(|h| Cell::from(*h).style(header_style));

            let header = Row::new(header_cells)
                .style(Style::default().bg(Color::Blue))
                .height(1);

            let selected_row_style = Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(app.colors.selected_row_style_fg);

            let rows = app.items.iter().enumerate().map(|(i, item)| {
                let color = match i % 2 {
                    0 => app.colors.normal_row_color,
                    _ => app.colors.alt_row_color,
                };
                let color = if app.selected_items.contains(&i) {
                    app.colors.selected_cell_style_fg
                } else {
                    color
                };
                let cells = item.iter().map(|c| Cell::from(c.clone()));
                Row::new(cells)
                    .style(Style::new().fg(app.colors.row_fg).bg(color))
                    .height(1)
            });

            let widths = [
                Constraint::Percentage(10),
                Constraint::Percentage(70),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
            ];

            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .row_highlight_style(selected_row_style);

            f.render_stateful_widget(table, size, &mut app.state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') | KeyCode::Down if shift_pressed => {
                        app.select();
                        app.next();
                    }
                    KeyCode::Char('k') | KeyCode::Up if shift_pressed => {
                        app.select();
                        app.previous();
                    }
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
