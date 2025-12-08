//! Deprecated file utilities
//!
//! This module is deprecated. Use the `paths` module for file path utilities
//! and `Session::load()/save()` for session persistence.

use anyhow::{Context, Result};
use std::fs::File;
use std::path::PathBuf;

/// Open the recent repositories file for reading
#[deprecated(
    since = "0.2.0",
    note = "Use paths::recent_repositories_path() instead"
)]
pub fn open_recent_repositories_file() -> Result<File> {
    File::open(".gh-pr-lander.repos.json")
        .context("Failed to open recent repositories file (.gh-pr-lander.repos.json)")
}

/// Create the recent repositories file for writing
#[deprecated(
    since = "0.2.0",
    note = "Use paths::recent_repositories_path() instead"
)]
pub fn create_recent_repositories_file() -> Result<File> {
    File::create(".gh-pr-lander.repos.json")
        .context("Failed to create recent repositories file (.gh-pr-lander.repos.json)")
}

/// Open the session state file for reading
#[deprecated(since = "0.2.0", note = "Use Session::load() instead")]
pub fn open_session_file() -> Result<File> {
    File::open(".session.json").context("Failed to open session state file (.session.json)")
}

/// Create the session state file for writing
#[deprecated(since = "0.2.0", note = "Use Session::save() instead")]
pub fn create_session_file() -> Result<File> {
    File::create(".session.json").context("Failed to create session state file (.session.json)")
}

/// Get the path to the API cache directory and ensure it exists
#[deprecated(since = "0.2.0", note = "Use paths::cache_dir() instead")]
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = std::env::current_dir()?.join(".cache");
    std::fs::create_dir_all(&cache_dir).context("Failed to create cache directory (.cache)")?;
    Ok(cache_dir)
}

/// Get the path to the API cache file
#[deprecated(since = "0.2.0", note = "Use paths::api_cache_path() instead")]
pub fn get_cache_file_path() -> Result<PathBuf> {
    #[allow(deprecated)]
    let cache_dir = get_cache_dir()?;
    Ok(cache_dir.join("gh-api-cache.json"))
}
