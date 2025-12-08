//! Multi-host GitHub client manager
//!
//! Manages GitHub API clients for different hosts (github.com, GitHub Enterprise).
//! Clients are lazily initialized and cached per host.

use crate::{ApiCache, CacheMode, CachedGitHubClient, OctocrabClient, DEFAULT_HOST};
use anyhow::{Context, Result};
use log::{debug, info};
use octocrab::Octocrab;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Resolves GitHub tokens for different hosts
///
/// Tries multiple sources in order:
/// 1. Host-specific env var (e.g., `GITHUB_TOKEN_GHE_EXAMPLE_COM`)
/// 2. `gh auth token --hostname {host}` command
/// 3. Generic `GITHUB_TOKEN` or `GH_TOKEN` (github.com only)
#[derive(Debug, Clone)]
pub struct TokenResolver {
    /// Cached default token from GITHUB_TOKEN/GH_TOKEN
    default_token: Option<String>,
}

impl Default for TokenResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenResolver {
    /// Create a new token resolver
    pub fn new() -> Self {
        let default_token = std::env::var("GITHUB_TOKEN")
            .or_else(|_| std::env::var("GH_TOKEN"))
            .ok();

        Self { default_token }
    }

    /// Get a token for the given host
    ///
    /// # Arguments
    ///
    /// * `host` - The GitHub host (None = github.com)
    ///
    /// # Token Resolution Order
    ///
    /// 1. `GITHUB_TOKEN_{HOST}` env var (e.g., `GITHUB_TOKEN_GHE_EXAMPLE_COM`)
    /// 2. `gh auth token --hostname {host}` command
    /// 3. `GITHUB_TOKEN` or `GH_TOKEN` (github.com only)
    pub async fn get_token(&self, host: Option<&str>) -> Result<String> {
        let host = host.unwrap_or(DEFAULT_HOST);

        // Try host-specific env var
        let env_key = format!(
            "GITHUB_TOKEN_{}",
            host.replace(['.', '-'], "_").to_uppercase()
        );
        if let Ok(token) = std::env::var(&env_key) {
            debug!("Using token from env var {} for host {}", env_key, host);
            return Ok(token);
        }

        // Try gh CLI with hostname
        debug!("Trying gh auth token for host {}", host);
        let output = tokio::process::Command::new("gh")
            .args(["auth", "token", "--hostname", host])
            .output()
            .await
            .context("Failed to run 'gh auth token'")?;

        if output.status.success() {
            let token = String::from_utf8(output.stdout)
                .context("Invalid UTF-8 in gh auth token output")?
                .trim()
                .to_string();
            if !token.is_empty() {
                debug!("Using token from gh CLI for host {}", host);
                return Ok(token);
            }
        }

        // Fallback to default token (for github.com only)
        if host == DEFAULT_HOST {
            if let Some(ref token) = self.default_token {
                debug!("Using default token (GITHUB_TOKEN/GH_TOKEN) for github.com");
                return Ok(token.clone());
            }
        }

        Err(anyhow::anyhow!(
            "No token found for host '{}'. \
             Set {} or run 'gh auth login --hostname {}'",
            host,
            env_key,
            host
        ))
    }
}

/// Manages GitHub API clients for multiple hosts
///
/// Lazily creates and caches clients per host. Each client is configured
/// with the appropriate base URL and authentication token.
///
/// # Example
///
/// ```rust,ignore
/// use gh_client::{ClientManager, ApiCache};
/// use std::sync::{Arc, Mutex};
///
/// let cache = Arc::new(Mutex::new(ApiCache::default()));
/// let mut manager = ClientManager::new(cache);
///
/// // Get client for github.com
/// let client = manager.get_client(None).await?;
///
/// // Get client for enterprise host
/// let ghe_client = manager.get_client(Some("ghe.example.com")).await?;
/// ```
pub struct ClientManager {
    /// Cached clients per host
    clients: HashMap<String, CachedGitHubClient<OctocrabClient>>,
    /// Shared API cache
    cache: Arc<Mutex<ApiCache>>,
    /// Token resolver
    tokens: TokenResolver,
    /// Default cache mode for new clients
    cache_mode: CacheMode,
}

impl ClientManager {
    /// Create a new client manager with the given cache
    pub fn new(cache: Arc<Mutex<ApiCache>>) -> Self {
        Self {
            clients: HashMap::new(),
            cache,
            tokens: TokenResolver::new(),
            cache_mode: CacheMode::ReadWrite,
        }
    }

    /// Create a new client manager with a specific cache mode
    pub fn with_cache_mode(cache: Arc<Mutex<ApiCache>>, cache_mode: CacheMode) -> Self {
        Self {
            clients: HashMap::new(),
            cache,
            tokens: TokenResolver::new(),
            cache_mode,
        }
    }

    /// Get or create a client for the given host
    ///
    /// # Arguments
    ///
    /// * `host` - The GitHub host (None = github.com)
    ///
    /// # Returns
    ///
    /// A cached GitHub client for the host
    pub async fn get_client(
        &mut self,
        host: Option<&str>,
    ) -> Result<&CachedGitHubClient<OctocrabClient>> {
        let key = host.unwrap_or(DEFAULT_HOST).to_string();

        if !self.clients.contains_key(&key) {
            let client = self.create_client(host).await?;
            self.clients.insert(key.clone(), client);
        }

        Ok(self.clients.get(&key).unwrap())
    }

    /// Get a mutable reference to the client for the given host
    pub async fn get_client_mut(
        &mut self,
        host: Option<&str>,
    ) -> Result<&mut CachedGitHubClient<OctocrabClient>> {
        let key = host.unwrap_or(DEFAULT_HOST).to_string();

        if !self.clients.contains_key(&key) {
            let client = self.create_client(host).await?;
            self.clients.insert(key.clone(), client);
        }

        Ok(self.clients.get_mut(&key).unwrap())
    }

    /// Check if a client exists for the given host (without creating one)
    pub fn has_client(&self, host: Option<&str>) -> bool {
        let key = host.unwrap_or(DEFAULT_HOST);
        self.clients.contains_key(key)
    }

    /// Remove a client for the given host
    ///
    /// This can be useful for forcing re-authentication after token changes.
    pub fn remove_client(&mut self, host: Option<&str>) {
        let key = host.unwrap_or(DEFAULT_HOST);
        self.clients.remove(key);
    }

    /// Get a clone of a client for the given host (for use in async tasks)
    ///
    /// Unlike `get_client`, this returns an owned client that can be moved
    /// into async tasks without borrowing from the manager.
    pub async fn clone_client(
        &mut self,
        host: Option<&str>,
    ) -> Result<CachedGitHubClient<OctocrabClient>> {
        // Ensure client exists
        let _ = self.get_client(host).await?;
        let key = host.unwrap_or(DEFAULT_HOST);
        Ok(self.clients.get(key).unwrap().clone())
    }

    /// Get the default token (github.com) if available
    pub fn default_token(&self) -> Option<&str> {
        self.tokens.default_token.as_deref()
    }

    /// Create a new client for the given host
    async fn create_client(
        &self,
        host: Option<&str>,
    ) -> Result<CachedGitHubClient<OctocrabClient>> {
        let effective_host = host.unwrap_or(DEFAULT_HOST);
        info!("Creating GitHub client for host: {}", effective_host);

        // Get token for this host
        let token = self.tokens.get_token(host).await?;

        // Build octocrab with appropriate base URI
        let mut builder = Octocrab::builder().personal_token(token);

        let base_url = if let Some(h) = host {
            if h != DEFAULT_HOST {
                let uri = format!("https://{}/api/v3", h);
                builder = builder.base_uri(&uri).context("Failed to set base URI")?;
                uri
            } else {
                "https://api.github.com".to_string()
            }
        } else {
            "https://api.github.com".to_string()
        };

        let octocrab = builder.build().context("Failed to build Octocrab client")?;
        let octocrab_client = OctocrabClient::with_base_url(Arc::new(octocrab), base_url);
        let cached =
            CachedGitHubClient::new(octocrab_client, Arc::clone(&self.cache), self.cache_mode);

        info!("GitHub client created for host: {}", effective_host);
        Ok(cached)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_resolver_env_key_generation() {
        // Test that host names are properly converted to env var format
        let hosts = [
            ("github.com", "GITHUB_TOKEN_GITHUB_COM"),
            ("ghe.example.com", "GITHUB_TOKEN_GHE_EXAMPLE_COM"),
            (
                "github-enterprise.corp.com",
                "GITHUB_TOKEN_GITHUB_ENTERPRISE_CORP_COM",
            ),
        ];

        for (host, expected_key) in hosts {
            let env_key = format!(
                "GITHUB_TOKEN_{}",
                host.replace(['.', '-'], "_").to_uppercase()
            );
            assert_eq!(
                env_key, expected_key,
                "Host '{}' should produce key '{}'",
                host, expected_key
            );
        }
    }

    #[test]
    fn test_client_manager_new() {
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let manager = ClientManager::new(cache);

        assert!(!manager.has_client(None));
        assert!(!manager.has_client(Some(DEFAULT_HOST)));
        assert!(!manager.has_client(Some("ghe.example.com")));
    }

    #[test]
    fn test_client_manager_with_cache_mode() {
        let cache = Arc::new(Mutex::new(ApiCache::default()));
        let manager = ClientManager::with_cache_mode(cache, CacheMode::WriteOnly);

        assert_eq!(manager.cache_mode, CacheMode::WriteOnly);
    }
}
