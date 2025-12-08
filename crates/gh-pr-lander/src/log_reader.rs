//! File Log Reader
//!
//! Reads log files with tailing support for the debug console.
//! Returns only new lines (delta) for efficient updates.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// Reads log file with tailing support
///
/// Returns only new lines since last poll (delta updates).
/// The caller (state) maintains its own ring buffer.
pub struct FileLogReader {
    path: PathBuf,
    last_position: u64,
    max_initial_lines: usize,
}

impl FileLogReader {
    /// Create a new log reader
    ///
    /// # Arguments
    /// * `path` - Path to the log file
    /// * `max_initial_lines` - Maximum lines to read on initial load
    pub fn new(path: PathBuf, max_initial_lines: usize) -> Self {
        Self {
            path,
            last_position: 0,
            max_initial_lines,
        }
    }

    /// Read initial content (last N lines)
    ///
    /// Returns the initial lines to populate the view
    pub fn read_initial(&mut self) -> std::io::Result<Vec<String>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(&file);

        // Read all lines, keep last max_initial_lines
        let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let start = all_lines.len().saturating_sub(self.max_initial_lines);
        let initial_lines = all_lines[start..].to_vec();

        // Remember position for tailing
        self.last_position = file.metadata()?.len();

        Ok(initial_lines)
    }

    /// Poll for new lines (non-blocking)
    ///
    /// Returns only the NEW lines since last poll (delta)
    pub fn poll_new_lines(&mut self) -> std::io::Result<Vec<String>> {
        let mut file = File::open(&self.path)?;
        let current_size = file.metadata()?.len();

        if current_size <= self.last_position {
            return Ok(Vec::new()); // No new content
        }

        file.seek(SeekFrom::Start(self.last_position))?;
        let reader = BufReader::new(file);

        let new_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

        self.last_position = current_size;
        Ok(new_lines)
    }

    /// Reset position to re-read from start
    pub fn clear(&mut self) {
        self.last_position = 0;
    }
}
