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
mod dispatcher;
mod logger;
mod middleware;
mod reducer;
mod reducers;
mod state;
mod store;
mod theme;
mod view_models;
mod views;

use actions::Action;
use middleware::{keyboard::KeyboardMiddleware, logging::LoggingMiddleware};
use state::AppState;
use store::Store;

fn main() -> io::Result<()> {
    // Initialize custom logger
    let logger = logger::init();

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
    store.add_middleware(Box::new(LoggingMiddleware::new()));
    store.add_middleware(Box::new(KeyboardMiddleware::new()));

    // Connect logger to dispatcher (so logs can be sent to debug console)
    logger.set_dispatcher(store.dispatcher().clone());

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
    loop {
        // Render
        terminal.draw(|mut frame| {
            let area = frame.area();
            views::render(store.state(), area, &mut frame);
        })?;

        // Check if we should quit
        if !store.state().running {
            break;
        }

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events (ignore key release)
                if key.kind == KeyEventKind::Press {
                    store.dispatch(Action::GlobalKeyPressed(key));
                }
            }
        }
    }

    Ok(())
}
