//! Configuration and data directory paths
//!
//! Uses XDG directories via `dirs` crate with fallbacks.
//!
//! Platform-specific locations:
//! - Linux: `~/.config/gh-pr-lander/`, `~/.cache/gh-pr-lander/`
//! - macOS: `~/Library/Application Support/gh-pr-lander/`, `~/Library/Caches/gh-pr-lander/`
//! - Windows: `%APPDATA%\gh-pr-lander\`, `%LOCALAPPDATA%\gh-pr-lander\`

use anyhow::{Context, Result};
use std::path::PathBuf;

const APP_NAME: &str = "gh-pr-lander";
const LOCAL_SESSION_FILE: &str = ".gh-pr-lander.session.toml";

/// Get the application config directory
/// Returns ~/.config/gh-pr-lander/ on Linux, ~/Library/Application Support/gh-pr-lander/ on macOS
pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("Could not determine config directory")?;
    let dir = base.join(APP_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Get the application cache directory
/// Returns ~/.cache/gh-pr-lander/ on Linux, ~/Library/Caches/gh-pr-lander/ on macOS
pub fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().context("Could not determine cache directory")?;
    let dir = base.join(APP_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Get path to global session file
pub fn global_session_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("session.toml"))
}

/// Get path to local session file (in CWD)
pub fn local_session_path() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join(LOCAL_SESSION_FILE))
}

/// Check if local session file exists
pub fn has_local_session() -> bool {
    local_session_path().map(|p| p.exists()).unwrap_or(false)
}

/// Get path to recent repositories file
pub fn recent_repositories_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("recent-repositories.toml"))
}

/// Get path to API cache file
pub fn api_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("gh-api-cache.json"))
}

/// Get path to app config file
pub fn app_config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_exists() {
        let dir = config_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.ends_with(APP_NAME));
    }

    #[test]
    fn test_cache_dir_exists() {
        let dir = cache_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.ends_with(APP_NAME));
    }

    #[test]
    fn test_session_paths() {
        let global = global_session_path().unwrap();
        assert!(global.ends_with("session.toml"));

        let local = local_session_path().unwrap();
        assert!(local.ends_with(LOCAL_SESSION_FILE));
    }
}
