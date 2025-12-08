use crate::paths;

/// Load config file content from XDG config directory
///
/// Config location:
/// - Linux: `~/.config/gh-pr-lander/config.toml`
/// - macOS: `~/Library/Application Support/gh-pr-lander/config.toml`
///
/// Returns the file content if found, None otherwise.
pub fn load_config_file() -> Option<String> {
    let config_path = paths::app_config_path().ok()?;
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            log::debug!("Loaded config from {}", config_path.display());
            Some(content)
        }
        Err(_) => None,
    }
}
