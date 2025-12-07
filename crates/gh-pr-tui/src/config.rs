use serde::{Deserialize, Serialize};
use std::env;

/// Application configuration loaded from gh-pr-tui.toml
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_ide_command")]
    pub ide_command: String,
    #[serde(default = "default_temp_dir")]
    pub temp_dir: String,
    #[serde(default = "default_approval_message")]
    pub approval_message: String,
}

fn default_ide_command() -> String {
    "code".to_string() // Default to VS Code
}

fn default_temp_dir() -> String {
    env::temp_dir()
        .join("gh-pr-tui")
        .to_string_lossy()
        .to_string()
}

fn default_approval_message() -> String {
    ":rocket: thanks for your contribution".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ide_command: default_ide_command(),
            temp_dir: default_temp_dir(),
            approval_message: default_approval_message(),
        }
    }
}

impl Config {
    /// Load config from CWD first, then home directory, or use defaults
    pub fn load() -> Self {
        if let Some(content) = gh_pr_config::load_config_file() {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }

        log::debug!("Using default config");
        Self::default()
    }
}
