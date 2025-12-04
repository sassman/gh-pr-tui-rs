//! File Log Reader
//!
//! Reads log files with tailing support for the debug console.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// Reads log file with tailing support
pub struct FileLogReader {
    path: PathBuf,
    last_position: u64,
    lines: Vec<String>,
    max_lines: usize,
}

impl FileLogReader {
    /// Create a new log reader
    ///
    /// # Arguments
    /// * `path` - Path to the log file
    /// * `max_lines` - Maximum number of lines to keep in memory
    pub fn new(path: PathBuf, max_lines: usize) -> Self {
        Self {
            path,
            last_position: 0,
            lines: Vec::with_capacity(max_lines.min(10_000)),
            max_lines,
        }
    }

    /// Read initial content (last N lines)
    pub fn read_initial(&mut self) -> std::io::Result<()> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(&file);

        // Read all lines, keep last max_lines
        let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let start = all_lines.len().saturating_sub(self.max_lines);
        self.lines = all_lines[start..].to_vec();

        // Remember position for tailing
        self.last_position = file.metadata()?.len();

        Ok(())
    }

    /// Poll for new lines (non-blocking)
    ///
    /// Returns the number of new lines read
    pub fn poll_new_lines(&mut self) -> std::io::Result<usize> {
        let mut file = File::open(&self.path)?;
        let current_size = file.metadata()?.len();

        if current_size <= self.last_position {
            return Ok(0); // No new content
        }

        file.seek(SeekFrom::Start(self.last_position))?;
        let reader = BufReader::new(file);

        let mut new_count = 0;
        for line in reader.lines() {
            if let Ok(line) = line {
                self.lines.push(line);
                new_count += 1;

                // Trim old lines if over max
                if self.lines.len() > self.max_lines {
                    self.lines.remove(0);
                }
            }
        }

        self.last_position = current_size;
        Ok(new_count)
    }

    /// Get all lines
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Total line count
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Clear lines (but keep tailing position)
    pub fn clear(&mut self) {
        self.lines.clear();
    }
}
