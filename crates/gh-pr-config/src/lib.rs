//! Configuration and file management for gh-pr-tui
//!
//! This crate provides:
//! - File path utilities for config and cache files
//! - Configuration file loading (TOML)
//! - Application configuration (AppConfig)
//! - Recent repositories persistence

pub mod app_config;
pub mod config_file;
pub mod files;
pub mod recent_repositories;

pub use app_config::AppConfig;
pub use config_file::load_config_file;
pub use files::{
    create_recent_repositories_file, create_session_file, get_cache_dir, get_cache_file_path,
    open_recent_repositories_file, open_session_file,
};
pub use recent_repositories::{load_recent_repositories, RecentRepository};
