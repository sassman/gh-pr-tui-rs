use anyhow::{Result, bail};
use log::debug;
use octocrab::{Octocrab, issues::IssueHandler, params};
use pr::Pr;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{KeyEvent, KeyModifiers},
        terminal,
    },
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
use std::{env, io};
use tokio::runtime::Runtime;

mod pr;

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
    prs: Vec<Pr>,
    recent_repos: Vec<Repo>,
    selected_repo: usize,
    filter: PrFilter,
    selected_prs: Vec<usize>,
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
            prs: Vec::new(),
            recent_repos: vec![
                Repo::new("cargo-generate", "cargo-generate", "main"),
                Repo::new("steganogram", "stegano-rs", "main"),
            ],
            selected_repo: 0,
            filter: PrFilter {
                title: "chore".to_string(),
            },
            selected_prs: Vec::new(),
            colors: TableColors::new(&PALETTES[0]),
        }
    }

    fn octocrab(&self) -> Result<Octocrab> {
        Ok(Octocrab::builder()
            .personal_token(
                env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN environment variable must be set"),
            )
            .build()?)
    }

    fn repo(&self) -> &Repo {
        &self.recent_repos[self.selected_repo]
    }

    /// Fetch data from GitHub for the selected repository and filter
    async fn fetch_data(&mut self) -> Result<()> {
        let github_data = fetch_github_data(&self.octocrab()?, &self.repo(), &self.filter).await?;
        self.prs = github_data;

        Ok(())
    }

    /// Move to the next PR in the list
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i < self.prs.len() - 1 {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Move to the previous PR in the list
    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Toggle the selection of the currently selected PR
    fn select_toggle(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if self.selected_prs.contains(&i) {
            self.selected_prs.retain(|&x| x != i);
        } else {
            self.selected_prs.push(i);
        }
    }

    /// todo: This should be opening a pop-up dialog to let the user type in a org, repo, and branch
    /// Here is the cheap version that just cycles through the recent repos
    async fn select_next_repo(&mut self) -> Result<()> {
        self.selected_repo = (self.selected_repo + 1) % (self.recent_repos.len() - 1);
        self.select_repo().await?;

        Ok(())
    }

    async fn select_repo(&mut self) -> Result<()> {
        // This function is a placeholder for future implementation
        // It could be used to select a specific repo from a list or input
        self.selected_prs.clear();
        self.fetch_data().await?;
        self.state.select(Some(0));
        debug!("Selecting repo: {}", self.repo().repo);
        Ok(())
    }

    /// Exit the application
    fn exit(&mut self) -> Result<()> {
        bail!("Exiting the application")
    }

    /// Rebase the selected PRs
    async fn rebase(&mut self) -> Result<()> {
        // for all selected PRs, authored by `dependabot` we rebase by adding the commend `@dependabot rebase`

        let octocrab = self.octocrab()?;
        for &pr_index in &self.selected_prs {
            if let Some(pr) = self.prs.get(pr_index) {
                if pr.author.starts_with("dependabot") {
                    debug!("Rebasing PR #{}", pr.number);

                    comment(&octocrab, self.repo(), pr, "@dependabot rebase").await?;
                } else {
                    debug!("Skipping PR #{} authored by {}", pr.number, pr.author);
                }
            } else {
                debug!("No PR found at index {}", pr_index);
            }
        }
        debug!("Rebasing selected PRs: {:?}", self.selected_prs);

        Ok(())
    }
}

async fn fetch_github_data<'a>(
    octocrab: &Octocrab,
    repo: &Repo,
    filter: &PrFilter,
) -> Result<Vec<Pr>> {
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

    let mut prs = Vec::new();

    for pr in page.items.into_iter().filter(|pr| {
        pr.title
            .as_ref()
            .unwrap_or(&"".to_string())
            .contains(&filter.title)
    }) {
        let pr = Pr::from_pull_request(&pr, repo, &octocrab).await;
        prs.push(pr);
    }

    Ok(prs)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    app.select_repo().await?;

    // Main loop
    loop {
        terminal.draw(|f| {
            let selected_repo = app.repo();
            let size = f.area();
            let block = Block::default()
                .title(format!(
                    "[/] GitHub PRs: {}/{}@{}",
                    &selected_repo.org, &selected_repo.repo, &selected_repo.branch
                ))
                .borders(Borders::ALL);

            let header_style = Style::default()
                .fg(app.colors.header_fg)
                .bg(app.colors.header_bg);

            let header_cells = ["#PR", "Description", "Author", "#Comments", "Mergable"]
                .iter()
                .map(|h| Cell::from(*h).style(header_style));

            let header = Row::new(header_cells)
                .style(Style::default().bg(Color::Blue))
                .height(1);

            let selected_row_style = Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(app.colors.selected_row_style_fg);

            let rows = app.prs.iter().enumerate().map(|(i, item)| {
                let color = match i % 2 {
                    0 => app.colors.normal_row_color,
                    _ => app.colors.alt_row_color,
                };
                let color = if app.selected_prs.contains(&i) {
                    app.colors.selected_cell_style_fg
                } else {
                    color
                };
                let row: Row = item.into();
                row.style(Style::new().fg(app.colors.row_fg).bg(color))
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

        if let Err(e) = handle_events(&mut app).await {
            debug!("Error handling events: {}", e);
            break;
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

async fn handle_events(app: &mut App) -> Result<()> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => handle_key_event(app, key).await,
        _ => Ok(()),
    }
}

async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    // let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Char('q') => app.exit(),
        KeyCode::Char('r') => app.rebase().await,
        KeyCode::Char('/') => app.select_next_repo().await,
        KeyCode::Char('j') | KeyCode::Down => {
            app.next();
            Ok(())
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.previous();
            Ok(())
        }
        KeyCode::Char(' ') => {
            app.select_toggle();
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn comment(octocrab: &Octocrab, repo: &Repo, pr: &Pr, body: &str) -> Result<()> {
    let issue = octocrab.issues(&repo.org, &repo.repo);
    issue.create_comment(pr.number as _, body).await?;

    Ok(())
}
