//! GitHub Actions Log Parser
//!
//! A library for parsing GitHub Actions workflow logs with ANSI color preservation
//! and GitHub Actions workflow command support (::group::, ::error::, ::warning::).
//!
//! # Example
//!
//! ```no_run
//! use gh_actions_log_parser::parse_workflow_logs;
//!
//! let zip_data: &[u8] = &[]; // ZIP file bytes from GitHub API
//! let parsed = parse_workflow_logs(zip_data)?;
//!
//! for job in &parsed.jobs {
//!     println!("Job: {}", job.name);
//!     for line in &job.lines {
//!         // Access styled segments, commands, group info, etc.
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod types;
mod ansi;
mod commands;
mod parser;

pub use types::*;
pub use parser::{parse_workflow_logs, job_log_to_tree};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        // Add tests here
    }
}
