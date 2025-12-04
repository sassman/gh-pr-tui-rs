//! File-based logging using simplelog
//!
//! Logs are written to a timestamped file in the current working directory.
//! The debug console reads from this file when opened.

use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::File;
use std::path::PathBuf;

/// Initialize file-based logging
///
/// Creates a log file with timestamp in the current directory.
/// Returns the path to the log file for use by the debug console.
pub fn init() -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let log_file = PathBuf::from(format!("debug-{}.log", timestamp));

    let level = std::env::var("RUST_LOG")
        .map(|v| match v.to_lowercase().as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        })
        .unwrap_or(LevelFilter::Debug);

    // Configure simplelog with timestamps
    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .set_time_offset_to_local()
        .unwrap_or_else(|c| c) // Fallback if local time offset fails
        .build();

    let file = File::create(&log_file).expect("Failed to create log file");

    WriteLogger::init(level, config, file).expect("Failed to initialize logger");

    log_file
}
