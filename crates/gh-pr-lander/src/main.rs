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
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

mod actions;
mod background;
mod capabilities;
mod dispatcher;
mod command_id;
mod commands;
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
use background::{spawn_background_worker, SharedState};
use middleware::{
    app_config_middleware::AppConfigMiddleware, bootstrap_middleware::BootstrapMiddleware,
    command_palette_middleware::CommandPaletteMiddleware,
    confirmation_popup_middleware::ConfirmationPopupMiddleware,
    debug_console_middleware::DebugConsoleMiddleware, github_middleware::GitHubMiddleware,
    keyboard_middleware::KeyboardMiddleware, navigation_middleware::NavigationMiddleware,
    pull_request_middleware::PullRequestMiddleware, repository_middleware::RepositoryMiddleware,
    text_input_middleware::TextInputMiddleware, Middleware,
};
use state::AppState;
use store::Store;

fn main() -> io::Result<()> {
    // Initialize file-based logger (returns log file path for debug console)
    let log_file = logger::init();

    log::info!("Starting GitHub PR Lander");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channels for thread communication
    let (action_tx, action_rx) = mpsc::channel::<Action>();
    let (result_tx, result_rx) = mpsc::channel::<Action>();

    // Create shared state for background thread to read
    let initial_state = AppState::default();
    let shared_state: SharedState = Arc::new(RwLock::new(initial_state.clone()));

    // Create store (main thread only, no middleware)
    let mut store = Store::new(initial_state);

    // Build middleware list (will run on background thread)
    let middleware: Vec<Box<dyn Middleware + Send>> = vec![
        Box::new(BootstrapMiddleware::new()),
        Box::new(AppConfigMiddleware::new()), // Load app config early
        Box::new(GitHubMiddleware::new()),    // GitHub client & API operations
        Box::new(KeyboardMiddleware::new()),
        // Translation middlewares - convert generic actions to view-specific actions
        Box::new(NavigationMiddleware::new()),
        Box::new(TextInputMiddleware::new()),
        // View-specific middlewares
        Box::new(CommandPaletteMiddleware::new()),
        Box::new(ConfirmationPopupMiddleware::new()),
        Box::new(RepositoryMiddleware::new()),
        Box::new(PullRequestMiddleware::new()), // Bulk loading coordination
        Box::new(DebugConsoleMiddleware::new(log_file)), // Debug console log reader
    ];

    // Spawn background worker with all middleware
    let bg_handle = spawn_background_worker(
        action_rx,         // Background receives actions
        action_tx.clone(), // For Dispatcher to re-enter middleware
        result_tx,         // Background sends results to reducers
        shared_state.clone(),
        middleware,
    );

    // Send bootstrap action to background
    action_tx
        .send(Action::Bootstrap(BootstrapAction::Start))
        .ok();

    // Main event loop
    let result = run_app(
        &mut terminal,
        &mut store,
        &action_tx,
        &result_rx,
        &shared_state,
    );

    // Graceful shutdown: signal background and wait
    drop(action_tx); // Close channel - recv() will return Err
    if let Err(e) = bg_handle.join() {
        log::error!("Background thread panicked: {:?}", e);
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {}", err);
    }

    log::info!("Exiting GitHub PR Lander");
    Ok(())
}

/// Maximum time budget for processing actions before rendering
/// This ensures smooth animations even when many actions are queued
const RENDER_BUDGET: Duration = Duration::from_millis(16); // ~60fps frame budget

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    store: &mut Store,
    action_tx: &mpsc::Sender<Action>,
    result_rx: &mpsc::Receiver<Action>,
    shared_state: &SharedState,
) -> io::Result<()> {
    loop {
        // === PHASE 1: Process results from background (time-budgeted) ===
        let start = Instant::now();
        let mut processed = 0;

        // try_recv() is non-blocking - returns immediately if empty
        while let Ok(result_action) = result_rx.try_recv() {
            log::trace!("Main loop: processing action: {:?}", result_action);

            // Events re-enter the middleware chain; other actions go to reducers
            match result_action {
                Action::Event(event) => {
                    // Re-inject event into middleware chain
                    log::trace!("Main loop: re-routing event to middleware: {:?}", event);
                    action_tx.send(Action::Event(event)).ok();
                }
                action => {
                    // Apply to reducer
                    store.dispatch(action);

                    // Sync state to shared (for background thread to read)
                    if let Ok(mut shared) = shared_state.write() {
                        *shared = store.state().clone();
                    }
                }
            }
            processed += 1;

            // Check time budget
            if start.elapsed() >= RENDER_BUDGET {
                break;
            }
        }

        if processed > 0 {
            log::debug!(
                "Main loop: processed {} results in {:?}",
                processed,
                start.elapsed()
            );
        }

        // === PHASE 2: Render ===
        let mut terminal_height = 0u16;
        terminal.draw(|frame| {
            let area = frame.area();
            terminal_height = area.height;
            views::render(store.state(), area, frame);
        })?;

        // Update debug console visible height based on terminal size
        // (70% of screen height minus 2 for borders)
        let debug_console_height = ((terminal_height as usize) * 70 / 100).saturating_sub(2);
        if store.state().debug_console.visible_height != debug_console_height {
            store.dispatch(Action::DebugConsole(
                crate::actions::DebugConsoleAction::SetVisibleHeight(debug_console_height),
            ));
        }

        // Update diff viewer viewport height based on terminal size
        // (full height minus 3 for status bar and borders)
        let terminal_width = terminal.size()?.width;
        let diff_viewport_height = terminal_height.saturating_sub(3) as usize;
        if let Some(ref inner) = store.state().diff_viewer.inner {
            if inner.viewport_height != diff_viewport_height {
                store.dispatch(Action::DiffViewer(
                    crate::actions::DiffViewerAction::SetViewport {
                        width: terminal_width,
                        height: diff_viewport_height as u16,
                    },
                ));
            }
        }

        // === PHASE 3: Check quit condition ===
        if !store.state().running {
            // Signal background to shutdown
            action_tx.send(Action::Global(GlobalAction::Quit)).ok();
            break;
        }

        // === PHASE 4: Handle user input ===
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events (ignore key release)
                if key.kind == KeyEventKind::Press {
                    // Send to background for middleware processing
                    action_tx
                        .send(Action::Global(GlobalAction::KeyPressed(key)))
                        .ok();
                }
            }
        }
    }

    Ok(())
}
