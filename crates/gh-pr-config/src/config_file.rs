use std::{env, path::PathBuf};

const CONFIG_FILE: &str = ".gh-pr-lander.toml";

/// Load config file content from CWD first, then home directory
///
/// Searches for gh-pr-tui.toml in:
/// 1. Current working directory
/// 2. Home directory as .gh-pr-tui.toml
///
/// Returns the file content if found, None otherwise.
pub fn load_config_file() -> Option<String> {
    // Try current directory first
    if let Ok(content) = std::fs::read_to_string(CONFIG_FILE) {
        log::debug!("Loaded config from {}", CONFIG_FILE);
        return Some(content);
    }

    // Try home directory
    if let Some(home_config) = get_home_config_path() {
        if let Ok(content) = std::fs::read_to_string(&home_config) {
            log::debug!("Loaded config from {}", home_config.display());
            return Some(content);
        }
    }

    None
}

/// Get the path to the config file in the home directory
///
/// Returns ~/.gh-pr-tui.toml if HOME environment variable is set.
fn get_home_config_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(CONFIG_FILE))
}
