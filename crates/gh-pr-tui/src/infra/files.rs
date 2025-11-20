use anyhow::{Context, Result};
use std::fs::File;
use std::path::PathBuf;

/// Open the recent repositories file for reading
pub fn open_recent_repositories_file() -> Result<File> {
    File::open(".recent-repositories.json")
        .context("Failed to open recent repositories file (.recent-repositories.json)")
}

/// Create the recent repositories file for writing
pub fn create_recent_repositories_file() -> Result<File> {
    File::create(".recent-repositories.json")
        .context("Failed to create recent repositories file (.recent-repositories.json)")
}

/// Open the session state file for reading
pub fn open_session_file() -> Result<File> {
    File::open(".session.json").context("Failed to open session state file (.session.json)")
}

/// Create the session state file for writing
pub fn create_session_file() -> Result<File> {
    File::create(".session.json").context("Failed to create session state file (.session.json)")
}

/// Get the path to the API cache directory and ensure it exists
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = std::env::current_dir()?.join(".cache");
    std::fs::create_dir_all(&cache_dir).context("Failed to create cache directory (.cache)")?;
    Ok(cache_dir)
}

/// Get the path to the API cache file
pub fn get_cache_file_path() -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    Ok(cache_dir.join("gh-api-cache.json"))
}
