//! Configuration and file management for gh-pr-tui
//!
//! This crate provides:
//! - File path utilities for config and cache files (via `paths` module)
//! - Configuration file loading (TOML)
//! - Application configuration (AppConfig)
//! - Session persistence (Session)
//! - Recent repositories persistence

/// Default GitHub host (public GitHub)
pub const DEFAULT_HOST: &str = "github.com";

pub mod app_config;
pub mod config_file;
pub mod files; // Deprecated: use `paths` module instead
pub mod paths;
pub mod recent_repositories;
pub mod session;

pub use app_config::AppConfig;
pub use config_file::load_config_file;
pub use paths::{
    api_cache_path, app_config_path, cache_dir, config_dir, global_session_path,
    has_local_session, local_session_path, recent_repositories_path,
};
pub use recent_repositories::{load_recent_repositories, save_recent_repositories, RecentRepository};
pub use session::Session;

// Re-export deprecated functions for backward compatibility
#[allow(deprecated)]
#[deprecated(since = "0.2.0", note = "Use paths::api_cache_path() instead")]
pub use files::{get_cache_dir, get_cache_file_path};
#[allow(deprecated)]
#[deprecated(since = "0.2.0", note = "Use Session::load()/save() instead")]
pub use files::{create_session_file, open_session_file};
#[allow(deprecated)]
#[deprecated(
    since = "0.2.0",
    note = "Use paths::recent_repositories_path() instead"
)]
pub use files::{create_recent_repositories_file, open_recent_repositories_file};
