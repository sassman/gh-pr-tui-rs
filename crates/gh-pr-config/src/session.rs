//! Session state persistence
//!
//! Handles loading and saving session state with local/global precedence.
//!
//! # Precedence
//!
//! 1. `$CWD/.gh-pr-lander.session.toml` - Local session (highest priority)
//! 2. `~/.config/gh-pr-lander/session.toml` - Global session (fallback)
//!
//! On save: Use local file if it exists, otherwise use global.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::paths;
use crate::DEFAULT_HOST;

const SESSION_VERSION: u32 = 1;

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub last_modified: DateTime<Utc>,
    pub version: u32,
}

/// Session data - the actual persisted state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionData {
    pub selected_repo_org: Option<String>,
    pub selected_repo_name: Option<String>,
    pub selected_repo_branch: Option<String>,
    /// GitHub host (None = github.com)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_repo_host: Option<String>,
    /// Selected PR number (not index) - more stable across refreshes
    pub selected_pr_no: Option<usize>,
}

/// Complete session with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub meta: SessionMeta,
    #[serde(default)]
    pub session: SessionData,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            meta: SessionMeta {
                last_modified: Utc::now(),
                version: SESSION_VERSION,
            },
            session: SessionData::default(),
        }
    }
}

impl Session {
    /// Load session with precedence: local > global > default
    pub fn load() -> Self {
        // Try local first
        if paths::has_local_session() {
            if let Ok(path) = paths::local_session_path() {
                if let Ok(session) = Self::load_from_path(&path) {
                    log::info!("Loaded local session from {:?}", path);
                    return session;
                }
            }
        }

        // Try global
        if let Ok(path) = paths::global_session_path() {
            if path.exists() {
                if let Ok(session) = Self::load_from_path(&path) {
                    log::info!("Loaded global session from {:?}", path);
                    return session;
                }
            }
        }

        log::info!("No existing session found, using defaults");
        Self::default()
    }

    /// Load session from specific path
    fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read session file: {:?}", path))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse session file: {:?}", path))
    }

    /// Save session (to local if exists, otherwise global)
    pub fn save(&mut self) -> Result<()> {
        // Update timestamp
        self.meta.last_modified = Utc::now();

        let path = if paths::has_local_session() {
            paths::local_session_path()?
        } else {
            paths::global_session_path()?
        };

        self.save_to_path(&path)
    }

    /// Save session to specific path
    fn save_to_path(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize session")?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, content)
            .with_context(|| format!("Failed to write session file: {:?}", path))?;

        log::info!("Saved session to {:?}", path);
        Ok(())
    }

    /// Update selected repository (with optional host)
    pub fn set_selected_repo(&mut self, org: &str, name: &str, branch: &str, host: Option<&str>) {
        self.session.selected_repo_org = Some(org.to_string());
        self.session.selected_repo_name = Some(name.to_string());
        self.session.selected_repo_branch = Some(branch.to_string());
        // Normalize github.com to None
        self.session.selected_repo_host = host
            .filter(|h| *h != DEFAULT_HOST && !h.is_empty())
            .map(|h| h.to_string());
    }

    /// Update selected PR number
    pub fn set_selected_pr_no(&mut self, pr_no: usize) {
        self.session.selected_pr_no = Some(pr_no);
    }

    /// Get selected repository as tuple (org, name, branch, host)
    pub fn selected_repo(&self) -> Option<(&str, &str, &str, Option<&str>)> {
        match (
            &self.session.selected_repo_org,
            &self.session.selected_repo_name,
            &self.session.selected_repo_branch,
        ) {
            (Some(org), Some(name), Some(branch)) => {
                Some((org, name, branch, self.session.selected_repo_host.as_deref()))
            }
            _ => None,
        }
    }

    /// Get selected PR number
    pub fn selected_pr_no(&self) -> Option<usize> {
        self.session.selected_pr_no
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_session() {
        let session = Session::default();
        assert_eq!(session.meta.version, SESSION_VERSION);
        assert!(session.session.selected_repo_org.is_none());
    }

    #[test]
    fn test_set_selected_repo() {
        let mut session = Session::default();
        session.set_selected_repo("owner", "repo", "main", None);

        let (org, name, branch, host) = session.selected_repo().unwrap();
        assert_eq!(org, "owner");
        assert_eq!(name, "repo");
        assert_eq!(branch, "main");
        assert!(host.is_none());
    }

    #[test]
    fn test_set_selected_repo_with_host() {
        let mut session = Session::default();
        session.set_selected_repo("owner", "repo", "main", Some("ghe.example.com"));

        let (org, name, branch, host) = session.selected_repo().unwrap();
        assert_eq!(org, "owner");
        assert_eq!(name, "repo");
        assert_eq!(branch, "main");
        assert_eq!(host, Some("ghe.example.com"));
    }

    #[test]
    fn test_host_normalization() {
        let mut session = Session::default();
        // github.com should be normalized to None
        session.set_selected_repo("owner", "repo", "main", Some("github.com"));
        let (_, _, _, host) = session.selected_repo().unwrap();
        assert!(host.is_none());
    }

    #[test]
    fn test_session_serialization() {
        let mut session = Session::default();
        session.set_selected_repo("cargo-generate", "cargo-generate", "main", None);
        session.set_selected_pr_no(42);

        let toml_str = toml::to_string_pretty(&session).unwrap();
        assert!(toml_str.contains("[meta]"));
        assert!(toml_str.contains("[session]"));
        assert!(toml_str.contains("cargo-generate"));
        // host should not be serialized when None
        assert!(!toml_str.contains("selected_repo_host"));

        // Round-trip
        let parsed: Session = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.selected_pr_no(), Some(42));
    }

    #[test]
    fn test_session_serialization_with_host() {
        let mut session = Session::default();
        session.set_selected_repo("org", "repo", "main", Some("ghe.example.com"));

        let toml_str = toml::to_string_pretty(&session).unwrap();
        assert!(toml_str.contains("ghe.example.com"));

        // Round-trip
        let parsed: Session = toml::from_str(&toml_str).unwrap();
        let (_, _, _, host) = parsed.selected_repo().unwrap();
        assert_eq!(host, Some("ghe.example.com"));
    }
}
