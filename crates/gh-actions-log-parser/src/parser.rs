//! Main parsing logic for GitHub Actions workflow logs

use crate::ansi::parse_ansi_line;
use crate::commands::parse_command;
use crate::types::{JobLog, LogLine, ParsedLog, WorkflowCommand};
use std::io::{Cursor, Read};
use thiserror::Error;
use zip::ZipArchive;

/// Errors that can occur during log parsing
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to read ZIP archive: {0}")]
    ZipError(#[from] zip::result::ZipError),

    #[error("Failed to read file from ZIP: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid UTF-8 in log content")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

/// Parse workflow logs from a ZIP file
///
/// GitHub Actions provides logs as a ZIP file where each job has its own log file.
/// This function extracts and parses all job logs with ANSI color preservation
/// and workflow command recognition.
///
/// # Arguments
///
/// * `zip_data` - Raw bytes of the ZIP file from GitHub Actions API
///
/// # Returns
///
/// A `ParsedLog` containing all jobs and their parsed log lines, or an error.
///
/// # Example
///
/// ```no_run
/// # use gh_actions_log_parser::parse_workflow_logs;
/// let zip_data: &[u8] = &[]; // From GitHub API
/// let parsed = parse_workflow_logs(zip_data)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_workflow_logs(zip_data: &[u8]) -> Result<ParsedLog, ParseError> {
    let cursor = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor)?;

    let mut jobs = Vec::new();

    // Process each file in the ZIP (each file is a job log)
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.name().to_string();

        // Skip directories
        if file.is_dir() {
            continue;
        }

        // Read the log file content
        let mut content_bytes = Vec::new();
        file.read_to_end(&mut content_bytes)?;
        let content = String::from_utf8(content_bytes)?;

        // Parse the job log
        let job_log = parse_job_log(&file_name, &content);
        jobs.push(job_log);
    }

    Ok(ParsedLog { jobs })
}

/// Parse a single job's log content
fn parse_job_log(job_name: &str, content: &str) -> JobLog {
    let mut lines = Vec::new();
    let mut group_tracker = GroupTracker::new();

    for raw_line in content.lines() {
        // Extract timestamp if present (GitHub Actions format)
        let (timestamp, line_content) = extract_timestamp(raw_line);

        // Parse ANSI codes to get styled segments
        let styled_segments = parse_ansi_line(line_content);

        // Get plain text for command parsing (without ANSI)
        let plain_text: String = styled_segments
            .iter()
            .map(|seg| seg.text.as_str())
            .collect();

        // Parse workflow command if present
        let command = match parse_command(&plain_text) {
            Some((cmd, _msg)) => {
                // Update group tracker based on command
                match &cmd {
                    WorkflowCommand::GroupStart { title } => {
                        group_tracker.enter_group(title.clone());
                    }
                    WorkflowCommand::GroupEnd => {
                        group_tracker.exit_group();
                    }
                    _ => {}
                }

                Some(cmd)
            }
            None => None,
        };

        // Get current group state
        let (group_level, group_title) = group_tracker.current_group();

        // Create log line
        lines.push(LogLine {
            content: line_content.to_string(), // Keep raw content with ANSI
            timestamp,
            styled_segments,
            command,
            group_level,
            group_title,
        });
    }

    JobLog {
        name: job_name.to_string(),
        lines,
    }
}

/// Extract timestamp from GitHub Actions log line format
///
/// GitHub Actions logs have timestamps in the format:
/// `2024-01-15T10:30:00.1234567Z some log line`
///
/// Returns (timestamp, content) where timestamp is Some if found, None otherwise.
fn extract_timestamp(line: &str) -> (Option<String>, &str) {
    // Check if line starts with ISO 8601 timestamp
    if line.len() > 30 {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() > 30
            && chars[4] == '-'
            && chars[7] == '-'
            && chars[10] == 'T'
            && chars[13] == ':'
            && chars[16] == ':'
            && (chars[19] == '.' || chars[19] == 'Z')
        {
            // Find where timestamp ends (look for 'Z' followed by space)
            if let Some(pos) = line.find("Z ") {
                let timestamp = line[..=pos].to_string(); // Include the 'Z'
                let content = &line[pos + 2..]; // Skip "Z " to get content
                return (Some(timestamp), content);
            }
        }
    }

    // No timestamp found
    (None, line)
}

/// Tracks group nesting state during parsing
struct GroupTracker {
    /// Stack of active groups (LIFO)
    stack: Vec<String>,
}

impl GroupTracker {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn enter_group(&mut self, title: String) {
        self.stack.push(title);
    }

    fn exit_group(&mut self) {
        self.stack.pop();
    }

    fn current_group(&self) -> (usize, Option<String>) {
        let level = self.stack.len();
        let title = self.stack.last().cloned();
        (level, title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_timestamp() {
        let line = "2024-01-15T10:30:00.1234567Z Running tests";
        let (ts, content) = extract_timestamp(line);
        assert_eq!(ts, Some("2024-01-15T10:30:00.1234567Z".to_string()));
        assert_eq!(content, "Running tests");
    }

    #[test]
    fn test_no_timestamp() {
        let line = "Just a regular log line";
        let (ts, content) = extract_timestamp(line);
        assert_eq!(ts, None);
        assert_eq!(content, "Just a regular log line");
    }

    #[test]
    fn test_group_tracker() {
        let mut tracker = GroupTracker::new();
        assert_eq!(tracker.current_group(), (0, None));

        tracker.enter_group("Build".to_string());
        assert_eq!(
            tracker.current_group(),
            (1, Some("Build".to_string()))
        );

        tracker.enter_group("Tests".to_string());
        assert_eq!(
            tracker.current_group(),
            (2, Some("Tests".to_string()))
        );

        tracker.exit_group();
        assert_eq!(
            tracker.current_group(),
            (1, Some("Build".to_string()))
        );

        tracker.exit_group();
        assert_eq!(tracker.current_group(), (0, None));
    }
}
