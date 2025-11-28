use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use log::{Level, Metadata, Record};
use std::fmt::Display;
use std::sync::{Arc, Mutex};

/// Custom logger that sends log messages to the debug console
pub struct DebugConsoleLogger {
    dispatcher: Arc<Mutex<Option<Dispatcher>>>,
    debug_mode: bool,
}

impl DebugConsoleLogger {
    pub fn new() -> Self {
        // Check DEBUG env var
        let debug_mode = std::env::var("DEBUG")
            .map(|v| v == "1" || v.to_lowercase().eq("true"))
            .unwrap_or(false);

        Self {
            dispatcher: Arc::new(Mutex::new(None)),
            debug_mode,
        }
    }

    /// Set the dispatcher after app initialization
    pub fn set_dispatcher(&self, dispatcher: Dispatcher) {
        if let Ok(mut d) = self.dispatcher.lock() {
            *d = Some(dispatcher);
        }
    }
}

impl log::Log for DebugConsoleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Enable all log levels
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Create owned log record
        let owned_record = OwnedLogRecord {
            ts: std::time::SystemTime::now(),
            level: record.level(),
            target: record.target().to_string(),
            message: format!("{}", record.args()),
        };

        // If DEBUG=1, also print to stderr (won't interfere with TUI)
        if self.debug_mode {
            eprintln!("{}", owned_record);
        }

        // Send to debug console
        if let Ok(dispatcher) = self.dispatcher.lock() {
            if let Some(ref d) = *dispatcher {
                d.dispatch(Action::DebugConsoleLogAdded(owned_record));
            }
        }
    }

    fn flush(&self) {}
}

use std::sync::OnceLock;
use std::time::SystemTime;

/// Global logger instance
static LOGGER: OnceLock<DebugConsoleLogger> = OnceLock::new();

/// Initialize the custom logger
pub fn init() -> &'static DebugConsoleLogger {
    let logger = LOGGER.get_or_init(DebugConsoleLogger::new);

    // Set as global logger
    log::set_logger(logger).expect("Failed to set logger");

    log::set_max_level(log::LevelFilter::Debug);

    logger
}

/// Owned log record (extracted from log::Record)
#[derive(Debug, Clone)]
pub struct OwnedLogRecord {
    pub ts: SystemTime,
    pub level: log::Level,
    pub target: String,
    pub message: String,
}

impl Display for OwnedLogRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let datetime: chrono::DateTime<chrono::Local> = self.ts.into();
        let timestamp = datetime.format("%H:%M:%S%.3f");
        write!(f, "[{}] [{}] {}", timestamp, self.level, self.message)
    }
}
