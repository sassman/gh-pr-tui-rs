//! Trait for fetching additional context lines.

use async_trait::async_trait;
use thiserror::Error;

/// Errors that can occur when fetching context.
#[derive(Debug, Error)]
pub enum ContextError {
    /// The requested file was not found.
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// A network error occurred.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Rate limited by the API.
    #[error("Rate limited, retry after {0} seconds")]
    RateLimited(u64),

    /// The context provider is not available.
    #[error("Context provider unavailable: {0}")]
    Unavailable(String),
}

/// Provides file content for context expansion.
///
/// Implement this trait to allow the diff viewer to fetch additional
/// lines of context beyond what's shown in the diff hunks.
///
/// # Example
///
/// ```ignore
/// struct GithubContextProvider {
///     client: GithubClient,
///     owner: String,
///     repo: String,
/// }
///
/// #[async_trait]
/// impl ContextProvider for GithubContextProvider {
///     async fn fetch_lines(
///         &self,
///         path: &str,
///         commit_sha: &str,
///         start_line: u32,
///         end_line: u32,
///     ) -> Result<Vec<String>, ContextError> {
///         let content = self.client
///             .get_file_contents(&self.owner, &self.repo, path, commit_sha)
///             .await
///             .map_err(|e| ContextError::NetworkError(e.to_string()))?;
///
///         let lines: Vec<String> = content.lines().map(String::from).collect();
///         Ok(lines.get((start_line - 1) as usize..end_line as usize)
///             .map(|s| s.to_vec())
///             .unwrap_or_default())
///     }
///
///     fn is_available(&self) -> bool {
///         true
///     }
/// }
/// ```
#[async_trait]
pub trait ContextProvider: Send + Sync {
    /// Fetch lines from a file at a specific commit.
    ///
    /// # Arguments
    /// * `path` - File path relative to repository root
    /// * `commit_sha` - The commit SHA to fetch from
    /// * `start_line` - 1-indexed start line (inclusive)
    /// * `end_line` - 1-indexed end line (inclusive)
    ///
    /// # Returns
    /// A vector of line contents (without newline characters).
    async fn fetch_lines(
        &self,
        path: &str,
        commit_sha: &str,
        start_line: u32,
        end_line: u32,
    ) -> Result<Vec<String>, ContextError>;

    /// Check if the provider is available (e.g., has valid credentials).
    fn is_available(&self) -> bool;
}

/// A no-op context provider for when context expansion is disabled.
#[allow(dead_code)]
pub struct NoOpContextProvider;

#[async_trait]
impl ContextProvider for NoOpContextProvider {
    async fn fetch_lines(
        &self,
        _path: &str,
        _commit_sha: &str,
        _start_line: u32,
        _end_line: u32,
    ) -> Result<Vec<String>, ContextError> {
        Err(ContextError::Unavailable(
            "Context expansion is disabled".to_string(),
        ))
    }

    fn is_available(&self) -> bool {
        false
    }
}
