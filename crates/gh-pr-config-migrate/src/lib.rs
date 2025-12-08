//! Migration utilities for gh-pr-lander configuration files
//!
//! This crate handles migrating configuration files from old formats/locations
//! to the new XDG-compliant structure. It can be removed once all users have migrated.
//!
//! # Migrations
//!
//! - `.session.json` → `~/.config/gh-pr-lander/session.toml`

use anyhow::{Context, Result};
use gh_pr_config::{global_session_path, Session};
use serde::Deserialize;
use std::fs;
use std::path::Path;

const OLD_SESSION_FILE: &str = ".session.json";

/// Old JSON session format for migration
#[derive(Debug, Deserialize)]
struct OldSessionJson {
    #[allow(dead_code)]
    selected_repository: Option<usize>,
    selected_pr: Option<usize>,
    // Old format stored just the index, not the full repo info
}

/// Run all migrations
///
/// This should be called during application bootstrap.
/// Migrations are idempotent - they only run if needed.
pub fn run_migrations() {
    if let Err(e) = migrate_session() {
        log::warn!("Session migration failed: {}", e);
    }
}

/// Migrate `.session.json` to TOML format
fn migrate_session() -> Result<()> {
    let old_path = Path::new(OLD_SESSION_FILE);

    // Skip if old file doesn't exist
    if !old_path.exists() {
        log::debug!("No old session file to migrate");
        return Ok(());
    }

    // Skip if new session already exists
    let new_path = global_session_path()?;
    if new_path.exists() {
        log::debug!("New session file already exists, skipping migration");
        // Clean up old file
        if let Err(e) = fs::remove_file(old_path) {
            log::warn!("Failed to remove old session file: {}", e);
        } else {
            log::info!("Removed old session file after migration");
        }
        return Ok(());
    }

    // Read old session
    let old_content = fs::read_to_string(old_path)
        .with_context(|| format!("Failed to read old session file: {:?}", old_path))?;

    let old_session: OldSessionJson =
        serde_json::from_str(&old_content).with_context(|| "Failed to parse old session JSON")?;

    log::info!("Migrating session from {:?} to {:?}", old_path, new_path);

    // Create new session with migrated data
    // Note: We can only migrate the PR index, not the repository selection
    // because the old format only stored the index, not the full repo info.
    // The repository selection will be lost in migration.
    let mut session = Session::default();
    if let Some(pr_idx) = old_session.selected_pr {
        session.set_selected_pr_no(pr_idx);
    }

    // Save new session
    session.save().context("Failed to save migrated session")?;

    // Remove old file
    if let Err(e) = fs::remove_file(old_path) {
        log::warn!("Failed to remove old session file: {}", e);
    } else {
        log::info!("Removed old session file after migration");
    }

    log::info!("Session migration completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_old_session_deserialize() {
        let json = r#"{"selected_repository": 2, "selected_pr": 5}"#;
        let old: OldSessionJson = serde_json::from_str(json).unwrap();
        assert_eq!(old.selected_repository, Some(2));
        assert_eq!(old.selected_pr, Some(5));
    }

    #[test]
    fn test_old_session_partial() {
        let json = r#"{}"#;
        let old: OldSessionJson = serde_json::from_str(json).unwrap();
        assert!(old.selected_repository.is_none());
        assert!(old.selected_pr.is_none());
    }
}
