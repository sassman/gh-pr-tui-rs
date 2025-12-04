use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, Event, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use std::io;

mod actions;
mod capabilities;
mod command_id;
mod commands;
mod dispatcher;
mod domain_models;
mod keybindings;
mod keymap;
mod log_reader;
mod logger;
mod middleware;
mod reducers;
mod state;
mod store;
mod utils;
mod view_models;
mod views;

use actions::{Action, BootstrapAction, GlobalAction};
use middleware::{
    app_config_middleware::AppConfigMiddleware, bootstrap_middleware::BootstrapMiddleware,
    command_palette_middleware::CommandPaletteMiddleware,
    confirmation_popup_middleware::ConfirmationPopupMiddleware,
    debug_console_middleware::DebugConsoleMiddleware, github_middleware::GitHubMiddleware,
    keyboard_middleware::KeyboardMiddleware, navigation_middleware::NavigationMiddleware,
    pull_request_middleware::PullRequestMiddleware, repository_middleware::RepositoryMiddleware,
    text_input_middleware::TextInputMiddleware,
};
use state::AppState;
use store::Store;

fn main() -> io::Result<()> {
    // Initialize file-based logger (returns log file path for debug console)
    let log_file = logger::init();

    log::info!("Starting gh-pr-lander");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize store with middleware
    let mut store = Store::new(AppState::default());

    // Add middleware in order (they execute in this order)
    store.add_middleware(Box::new(BootstrapMiddleware::new()));
    store.add_middleware(Box::new(AppConfigMiddleware::new())); // Load app config early
    store.add_middleware(Box::new(GitHubMiddleware::new())); // GitHub client & API operations
    store.add_middleware(Box::new(KeyboardMiddleware::new()));
    // Translation middlewares - convert generic actions to view-specific actions
    store.add_middleware(Box::new(NavigationMiddleware::new()));
    store.add_middleware(Box::new(TextInputMiddleware::new()));
    // View-specific middlewares
    store.add_middleware(Box::new(CommandPaletteMiddleware::new()));
    store.add_middleware(Box::new(ConfirmationPopupMiddleware::new()));
    store.add_middleware(Box::new(RepositoryMiddleware::new()));
    store.add_middleware(Box::new(PullRequestMiddleware::new())); // Bulk loading coordination
    store.add_middleware(Box::new(DebugConsoleMiddleware::new(log_file))); // Debug console log reader

    // Main event loop
    let result = run_app(&mut terminal, &mut store);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    log::info!("Exiting gh-pr-lander");
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    store: &mut Store,
) -> io::Result<()> {
    // Start bootstrap process
    store.dispatch(Action::Bootstrap(BootstrapAction::Start));

    loop {
        // Process any pending actions from background threads
        let pending = store.dispatcher().drain();
        for action in pending {
            store.dispatch(action);
        }

        // Render
        let mut terminal_height = 0u16;
        terminal.draw(|frame| {
            let area = frame.area();
            terminal_height = area.height;
            views::render(store.state(), area, frame);
        })?;

        // Dirty hack to fix the scrolling behaviour of the debug console
        // Update debug console visible height based on terminal size
        // (70% of screen height minus 2 for borders)
        let debug_console_height = ((terminal_height as usize) * 70 / 100).saturating_sub(2);
        if store.state().debug_console.visible_height != debug_console_height {
            store.dispatch(Action::DebugConsole(
                crate::actions::DebugConsoleAction::SetVisibleHeight(debug_console_height),
            ));
        }

        // Check if we should quit
        if !store.state().running {
            break;
        }

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events (ignore key release)
                if key.kind == KeyEventKind::Press {
                    store.dispatch(Action::Global(GlobalAction::KeyPressed(key)));
                }
            }
        }
    }

    Ok(())
}
