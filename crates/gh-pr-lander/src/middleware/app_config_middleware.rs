//! App Config Middleware
//!
//! Handles loading application configuration on bootstrap.

use crate::actions::{Action, BootstrapAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_pr_config::AppConfig;

/// Middleware for loading application configuration
pub struct AppConfigMiddleware {
    config_loaded: bool,
}

impl AppConfigMiddleware {
    pub fn new() -> Self {
        Self {
            config_loaded: false,
        }
    }
}

impl Default for AppConfigMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for AppConfigMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::Bootstrap(BootstrapAction::Start) => {
                if !self.config_loaded {
                    log::info!("AppConfigMiddleware: Loading application configuration");
                    // This can block - we're on the background thread
                    let config = AppConfig::load();
                    log::info!(
                        "AppConfigMiddleware: Loaded config (ide_command: {})",
                        config.ide_command
                    );
                    dispatcher.dispatch(Action::Bootstrap(BootstrapAction::ConfigLoaded(config)));
                    self.config_loaded = true;
                }
                true // Pass through
            }
            _ => true, // All other actions pass through
        }
    }
}
