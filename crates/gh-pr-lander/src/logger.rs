//! File-based logging using simplelog
//!
//! Log file location depends on build type:
//! - Debug builds: current working directory (for development convenience)
//! - Release builds: cache directory (~/.cache/gh-pr-lander/ on Linux)
//!
//! The debug console reads from this file when opened.

use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::fs::File;
use std::path::PathBuf;

/// Get the log file path based on build type
fn log_file_path() -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!("debug-{}.log", timestamp);

    if cfg!(debug_assertions) {
        // Debug build: log in current directory for convenience
        PathBuf::from(filename)
    } else {
        // Release build: log in cache directory
        gh_pr_config::cache_dir()
            .map(|dir| dir.join(&filename))
            .unwrap_or_else(|_| PathBuf::from(filename))
    }
}

/// Initialize file-based logging
///
/// Creates a log file with timestamp.
/// Returns the path to the log file for use by the debug console.
pub fn init() -> PathBuf {
    let log_file = log_file_path();

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
