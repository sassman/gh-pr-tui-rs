//! GitHub API client with caching support
//!
//! This crate provides a trait-based GitHub API client with optional caching.
//! The design follows the decorator pattern, allowing caching behavior to be
//! composed with the base client.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │              GitHubClient trait                  │
//! │  - fetch_pull_requests()                         │
//! │  - fetch_check_runs()                            │
//! │  - fetch_commit_status()                         │
//! └─────────────────────────────────────────────────┘
//!                        │
//!        ┌───────────────┴───────────────┐
//!        ▼                               ▼
//! ┌─────────────────┐         ┌─────────────────────┐
//! │ OctocrabClient  │         │ CachedGitHubClient  │
//! │ (direct API)    │◄────────│ (decorator)         │
//! └─────────────────┘         └─────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use gh_client::{GitHubClient, OctocrabClient, CachedGitHubClient, CacheMode};
//! use gh_api_cache::ApiCache;
//! use std::sync::{Arc, Mutex};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create octocrab instance
//! let octocrab = octocrab::Octocrab::builder()
//!     .personal_token("token".to_string())
//!     .build()?;
//!
//! // Direct client (no caching)
//! let direct = OctocrabClient::new(Arc::new(octocrab.clone()));
//!
//! // Cached client with full read/write caching
//! let cache = Arc::new(Mutex::new(ApiCache::default()));
//! let cached = CachedGitHubClient::new(
//!     OctocrabClient::new(Arc::new(octocrab)),
//!     cache,
//!     CacheMode::ReadWrite,
//! );
//!
//! // Both implement the same trait
//! let prs = cached.fetch_pull_requests("owner", "repo", None).await?;
//! # Ok(())
//! # }
//! ```

pub mod cached_client;
pub mod client;
pub mod client_manager;
pub mod octocrab_client;
pub mod types;

/// Default GitHub host (public GitHub)
pub const DEFAULT_HOST: &str = "github.com";

pub use cached_client::CachedGitHubClient;
pub use client::{CacheMode, GitHubClient};
pub use client_manager::{ClientManager, TokenResolver};
pub use octocrab_client::OctocrabClient;
pub use types::{
    CheckRun, CheckStatus, CiState, CiStatus, MergeMethod, MergeResult, PullRequest, ReviewComment,
    ReviewEvent, WorkflowRun, WorkflowRunConclusion, WorkflowRunStatus,
};

// Re-export cache types for convenience
pub use gh_api_cache::{ApiCache, CacheStats, CachedResponse};

// Re-export octocrab so consumers don't need to depend on it directly
pub use octocrab;

/// Re-exported octocrab types for convenience
///
/// This allows consumers to access octocrab types without adding
/// octocrab as a direct dependency.
pub mod octocrab_types {
    pub use octocrab::models;
    pub use octocrab::params;
    pub use octocrab::Octocrab;
}
