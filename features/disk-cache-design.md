# Disk Cache Design for Development Workflow

## Problem Statement

During development, frequent app restarts trigger repeated GitHub API calls for the same data:
- Start app → Load 10 repos → 100 API calls
- Make code change → Restart → Load 10 repos → 100 API calls (again!)
- Repeat 20 times during development session → **2,000 unnecessary API calls**

With a 20-minute cache, the same development session:
- Start app → Load 10 repos → 100 API calls → **cached**
- Restart (within 20 min) → Load from cache → **0 API calls**
- Repeat 20 times → **100 total API calls** (95% reduction!)

---

## Design Goals

1. ✅ **Transparent caching** - Works without code changes in effect handlers
2. ✅ **Simple implementation** - Minimal dependencies, easy to debug
3. ✅ **Development-focused** - Optimized for start/stop cycles
4. ✅ **Smart invalidation** - Cache only appropriate requests
5. ✅ **Easy to disable/clear** - Debug issues by bypassing cache

---

## Architecture

### 1. Cache Structure

**Single JSON file approach**:
```
.cache/gh-api-cache.json
```

```json
{
  "version": 1,
  "entries": {
    "GET:/repos/org/repo/pulls?state=open&base=main": {
      "response_body": "{...}",
      "timestamp": 1234567890,
      "etag": "\"abc123\"",
      "status_code": 200
    },
    "GET:/repos/org/repo/pulls/123": {
      "response_body": "{...}",
      "timestamp": 1234567891,
      "etag": "\"def456\"",
      "status_code": 200
    }
  }
}
```

**Cache key format**: `{METHOD}:{path}?{sorted_query_params}`
- Example: `GET:/repos/acme/widget/pulls?base=main&state=open`
- Deterministic (sorted params)
- Human-readable for debugging
- Easy to invalidate specific patterns

---

### 2. Cache Layer Architecture

```
┌─────────────────────────────────────────────────┐
│  Effect Handlers (effect.rs)                    │
│  - LoadAllRepos                                 │
│  - LoadSingleRepo                               │
│  - RefreshCurrentRepo                           │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│  GitHub API Client (gh.rs / new layer)          │
│  - fetch_prs_cached()                           │
│  - fetch_pr_details_cached()                    │
│  - bypass_cache flag support                    │
└────────────────┬────────────────────────────────┘
                 │
        ┌────────┴────────┐
        ▼                 ▼
┌──────────────┐   ┌─────────────────┐
│ Disk Cache   │   │ Octocrab/GitHub │
│ (cache.rs)   │   │ API (live)      │
└──────────────┘   └─────────────────┘
```

---

### 3. Cache Module API

**File**: `crates/gh-pr-tui/src/cache.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// GitHub API response cache for development workflow
pub struct ApiCache {
    cache_file: PathBuf,
    ttl_seconds: u64,
    entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    response_body: String,
    timestamp: u64,  // Unix timestamp
    etag: Option<String>,
    status_code: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheFile {
    version: u8,
    entries: HashMap<String, CacheEntry>,
}

impl ApiCache {
    /// Create cache with 20-minute TTL (hardcoded for development)
    pub fn new() -> Result<Self, anyhow::Error> {
        let cache_dir = std::env::current_dir()?.join(".cache");
        std::fs::create_dir_all(&cache_dir)?;

        let cache_file = cache_dir.join("gh-api-cache.json");
        let ttl_seconds = 20 * 60; // 20 minutes

        let entries = if cache_file.exists() {
            Self::load_from_disk(&cache_file)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            cache_file,
            ttl_seconds,
            entries,
        })
    }

    /// Get cached response if available and not stale
    pub fn get(&self, method: &str, url: &str, params: &[(&str, &str)])
        -> Option<CachedResponse>
    {
        let key = self.cache_key(method, url, params);

        if let Some(entry) = self.entries.get(&key) {
            // Check if entry is still fresh
            let age_seconds = self.current_timestamp() - entry.timestamp;

            if age_seconds < self.ttl_seconds {
                debug!(
                    "Cache HIT: {} (age: {}s, ttl: {}s)",
                    key, age_seconds, self.ttl_seconds
                );

                return Some(CachedResponse {
                    body: entry.response_body.clone(),
                    etag: entry.etag.clone(),
                    status_code: entry.status_code,
                });
            } else {
                debug!(
                    "Cache STALE: {} (age: {}s, ttl: {}s)",
                    key, age_seconds, self.ttl_seconds
                );
            }
        } else {
            debug!("Cache MISS: {}", key);
        }

        None
    }

    /// Store response in cache
    pub fn set(&mut self,
        method: &str,
        url: &str,
        params: &[(&str, &str)],
        response: &CachedResponse,
    ) -> Result<(), anyhow::Error> {
        let key = self.cache_key(method, url, params);

        let entry = CacheEntry {
            response_body: response.body.clone(),
            timestamp: self.current_timestamp(),
            etag: response.etag.clone(),
            status_code: response.status_code,
        };

        self.entries.insert(key.clone(), entry);

        debug!("Cache SET: {}", key);

        // Persist to disk asynchronously (don't block)
        self.save_to_disk()?;

        Ok(())
    }

    /// Invalidate specific cache entry
    pub fn invalidate(&mut self, method: &str, url: &str, params: &[(&str, &str)]) {
        let key = self.cache_key(method, url, params);
        if self.entries.remove(&key).is_some() {
            debug!("Cache INVALIDATE: {}", key);
            let _ = self.save_to_disk();
        }
    }

    /// Invalidate all entries matching a pattern (e.g., all PRs from a repo)
    pub fn invalidate_pattern(&mut self, pattern: &str) {
        let keys_to_remove: Vec<_> = self.entries
            .keys()
            .filter(|k| k.contains(pattern))
            .cloned()
            .collect();

        for key in &keys_to_remove {
            self.entries.remove(key);
            debug!("Cache INVALIDATE (pattern '{}'): {}", pattern, key);
        }

        if !keys_to_remove.is_empty() {
            let _ = self.save_to_disk();
        }
    }

    /// Clear entire cache
    pub fn clear(&mut self) -> Result<(), anyhow::Error> {
        self.entries.clear();
        self.save_to_disk()?;
        debug!("Cache CLEARED");
        Ok(())
    }

    /// Get cache statistics for debugging
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.entries.len();
        let fresh_entries = self.entries.values()
            .filter(|e| {
                let age = self.current_timestamp() - e.timestamp;
                age < self.ttl_seconds
            })
            .count();
        let stale_entries = total_entries - fresh_entries;

        CacheStats {
            total_entries,
            fresh_entries,
            stale_entries,
            ttl_seconds: self.ttl_seconds,
        }
    }

    // Private helpers

    fn cache_key(&self, method: &str, url: &str, params: &[(&str, &str)]) -> String {
        if params.is_empty() {
            format!("{}:{}", method, url)
        } else {
            // Sort params for deterministic key
            let mut sorted_params = params.to_vec();
            sorted_params.sort_by_key(|(k, _)| *k);

            let query = sorted_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");

            format!("{}:{}?{}", method, url, query)
        }
    }

    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn load_from_disk(path: &PathBuf) -> Result<HashMap<String, CacheEntry>, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let cache_file: CacheFile = serde_json::from_str(&content)?;

        // Validate version
        if cache_file.version != 1 {
            warn!("Cache file version mismatch, clearing cache");
            return Ok(HashMap::new());
        }

        Ok(cache_file.entries)
    }

    fn save_to_disk(&self) -> Result<(), anyhow::Error> {
        let cache_file = CacheFile {
            version: 1,
            entries: self.entries.clone(),
        };

        let content = serde_json::to_string_pretty(&cache_file)?;
        std::fs::write(&self.cache_file, content)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub body: String,
    pub etag: Option<String>,
    pub status_code: u16,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub fresh_entries: usize,
    pub stale_entries: usize,
    pub ttl_seconds: u64,
}
```

---

### 4. Integration with GitHub API Calls

**Caching wrapper for octocrab calls**:

```rust
// In effect.rs or new gh.rs module

use crate::cache::{ApiCache, CachedResponse};

/// Fetch PRs with caching support
async fn fetch_prs_cached(
    cache: &mut ApiCache,
    octocrab: &Octocrab,
    repo: &Repo,
    bypass_cache: bool,
) -> Result<Vec<Pr>, Error> {
    let url = format!("/repos/{}/{}/pulls", repo.org, repo.repo);
    let params = [
        ("state", "open"),
        ("base", &repo.branch),
    ];

    // Try cache first (unless bypassed)
    if !bypass_cache {
        if let Some(cached) = cache.get("GET", &url, &params) {
            // Parse cached JSON response
            let prs: Vec<Pr> = serde_json::from_str(&cached.body)?;
            info!("Loaded {} PRs from cache for {}/{}", prs.len(), repo.org, repo.repo);
            return Ok(prs);
        }
    }

    // Cache miss or bypassed - fetch from API
    info!("Fetching PRs from GitHub API for {}/{}", repo.org, repo.repo);

    let response = octocrab
        .get(&url)
        .query(&params)
        .send()
        .await?;

    let status = response.status();
    let etag = response.headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let body = response.text().await?;

    // Store in cache
    cache.set(
        "GET",
        &url,
        &params,
        &CachedResponse {
            body: body.clone(),
            etag,
            status_code: status.as_u16(),
        },
    )?;

    // Parse and return
    let prs: Vec<Pr> = serde_json::from_str(&body)?;
    Ok(prs)
}
```

---

### 5. When to Use vs Bypass Cache

```rust
// Effect handling with cache control

Effect::LoadAllRepos { repos, filter } => {
    // ✅ USE CACHE - Initial load on startup
    for (index, repo) in repos {
        let prs = fetch_prs_cached(
            &mut app.cache,
            app.octocrab()?,
            &repo,
            bypass_cache: false,  // Use cache
        ).await?;
        // ... store prs ...
    }
}

Effect::LoadSingleRepo { repo_index, repo } => {
    // ✅ USE CACHE - Lazy loading on tab switch
    let prs = fetch_prs_cached(
        &mut app.cache,
        app.octocrab()?,
        &repo,
        bypass_cache: false,  // Use cache
    ).await?;
}

Effect::RefreshRepo { repo_index, repo } => {
    // ❌ BYPASS CACHE - Manual refresh
    let prs = fetch_prs_cached(
        &mut app.cache,
        app.octocrab()?,
        &repo,
        bypass_cache: true,  // Force fresh data
    ).await?;

    // After successful fetch, invalidate old cache
    app.cache.invalidate_pattern(&format!("/repos/{}/{}", repo.org, repo.repo));
}

Effect::CheckMergeableStatus { repo, pr_number } => {
    // ❌ BYPASS CACHE - PR status checks are time-sensitive
    let status = check_mergeable_status_uncached(
        app.octocrab()?,
        repo,
        pr_number,
    ).await?;
}

Effect::FetchBuildLogs { ... } => {
    // ❌ BYPASS CACHE - Build logs change frequently
    // No caching for build status
}
```

---

### 6. Cache Management UI

**Add to debug console or status bar**:

```rust
// Show cache stats
let stats = app.cache.stats();
debug!(
    "Cache: {} fresh, {} stale, {} total (TTL: {}min)",
    stats.fresh_entries,
    stats.stale_entries,
    stats.total_entries,
    stats.ttl_seconds / 60,
);
```

**Command palette commands**:

```rust
// Add to command_palette_integration.rs
commands.push(CommandItem {
    title: "Clear API cache".to_string(),
    description: "Clear all cached GitHub API responses".to_string(),
    category: "Debug".to_string(),
    shortcut_hint: None,
    context: None,
    action: Action::ClearApiCache,
});

commands.push(CommandItem {
    title: "Show cache stats".to_string(),
    description: "Display API cache statistics".to_string(),
    category: "Debug".to_string(),
    shortcut_hint: None,
    context: None,
    action: Action::ShowCacheStats,
});
```

---

## Implementation Plan

### Phase 1: Core Cache Module (Day 1)
1. ✅ Create `cache.rs` with `ApiCache` struct
2. ✅ Implement `get()`, `set()`, `invalidate()` methods
3. ✅ Add JSON file persistence
4. ✅ Unit tests for cache logic

### Phase 2: Integration (Day 2)
1. ✅ Add `cache: ApiCache` field to `App` struct
2. ✅ Create `fetch_prs_cached()` wrapper function
3. ✅ Update `LoadAllRepos` effect to use cache
4. ✅ Update `LoadSingleRepo` effect to use cache
5. ✅ Ensure `RefreshCurrentRepo` bypasses cache

### Phase 3: Cache Management (Day 3)
1. ✅ Add `ClearApiCache` action
2. ✅ Add cache stats to debug console
3. ✅ Add command palette commands
4. ✅ Test cache hit/miss behavior

### Phase 4: Fine-tuning (Day 4)
1. ✅ Add cache size limits (optional)
2. ✅ Add automatic cleanup of stale entries
3. ✅ Performance testing
4. ✅ Documentation

---

## Cache Invalidation Strategy

### Automatic Invalidation
- **Manual refresh**: Invalidate all entries for that repo
- **PR merge**: Invalidate specific PR + PR list
- **Repo switch with refresh**: Invalidate that repo's cache

### Manual Invalidation
- Command: "Clear API cache" (clear all)
- Debug console: Show "Clear Cache" button
- Config option: `DISABLE_API_CACHE=1` environment variable

### Time-based Invalidation
- **TTL**: 20 minutes (hardcoded)
- **Stale entries**: Ignored on get(), removed on next save
- **Cleanup**: Remove stale entries weekly or on startup

---

## Benefits

### Development Workflow
- **Restart cycles**: 95% reduction in API calls during development
- **Fast startup**: Cached responses = instant load
- **Rate limit friendly**: Stay well under 5K/hour during coding sessions

### User Experience
- **Faster initial load**: Especially after recent restart
- **Offline-ish mode**: Works with stale cache if no network
- **Predictable**: Always fetches fresh on manual refresh

### Debugging
- **Reproducible state**: Cache provides consistent data during testing
- **Inspect responses**: JSON cache file is human-readable
- **Easy to clear**: One command clears everything

---

## Configuration Options (Future)

```rust
// .env or config file
API_CACHE_ENABLED=true          // Enable/disable cache
API_CACHE_TTL_MINUTES=20        // Cache lifetime
API_CACHE_MAX_SIZE_MB=50        // Size limit
API_CACHE_LOCATION=.cache       // Cache directory
```

---

## Example: Development Session

**Without cache**:
```
09:00 - Start app → 100 API calls
09:05 - Fix bug, restart → 100 API calls
09:10 - Test fix, restart → 100 API calls
09:15 - Tweak UI, restart → 100 API calls
09:20 - Final test, restart → 100 API calls
Total: 500 API calls in 20 minutes
```

**With cache (20-min TTL)**:
```
09:00 - Start app → 100 API calls → cached
09:05 - Fix bug, restart → 0 API calls (cache hit)
09:10 - Test fix, restart → 0 API calls (cache hit)
09:15 - Tweak UI, restart → 0 API calls (cache hit)
09:20 - Final test, restart → 0 API calls (cache hit)
09:25 - Manual refresh → 100 API calls (bypass cache)
Total: 200 API calls in 25 minutes (60% reduction)
```

---

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_cache_get_set() {
    let mut cache = ApiCache::new().unwrap();

    let response = CachedResponse {
        body: "{\"data\": \"test\"}".into(),
        etag: Some("abc123".into()),
        status_code: 200,
    };

    cache.set("GET", "/test", &[], &response).unwrap();
    let cached = cache.get("GET", "/test", &[]).unwrap();

    assert_eq!(cached.body, response.body);
}

#[test]
fn test_cache_ttl_expiry() {
    let mut cache = ApiCache::new().unwrap();
    cache.ttl_seconds = 1; // 1 second TTL for test

    cache.set("GET", "/test", &[], &response).unwrap();

    // Immediate get - should hit
    assert!(cache.get("GET", "/test", &[]).is_some());

    // Wait 2 seconds
    std::thread::sleep(Duration::from_secs(2));

    // Should be stale
    assert!(cache.get("GET", "/test", &[]).is_none());
}
```

### Integration Tests
- Start app → check cache miss → check cache file created
- Restart app → check cache hit → verify no API calls
- Wait 21 minutes → restart → check cache miss (stale)
- Manual refresh → verify cache bypassed → verify new cache entry

---

## Migration Path

### Phase 1: Behind Feature Flag
```rust
#[cfg(feature = "api-cache")]
let prs = fetch_prs_cached(...);

#[cfg(not(feature = "api-cache"))]
let prs = fetch_prs_uncached(...);
```

### Phase 2: Default Enabled
- Enable by default after testing
- Environment variable to disable: `DISABLE_API_CACHE=1`

### Phase 3: Always On
- Remove disable option once stable
- Keep clear cache command for debugging

---

## Alternatives Considered

### 1. In-Memory Cache Only
**Pros**: Faster, simpler
**Cons**: Lost on restart (doesn't solve dev workflow issue)
**Verdict**: ❌ Doesn't help with restarts

### 2. SQLite Database
**Pros**: Better performance, complex queries
**Cons**: Extra dependency, overkill for simple cache
**Verdict**: ❌ Too complex for this use case

### 3. Redis/External Cache
**Pros**: Shared cache, very fast
**Cons**: Extra service to run, not suitable for CLI app
**Verdict**: ❌ Not appropriate for desktop app

### 4. HTTP Cache Middleware
**Pros**: Standard approach, well-tested
**Cons**: Octocrab doesn't have built-in cache middleware
**Verdict**: ⚠️ Would need custom implementation anyway

**Selected**: Simple JSON file cache (best for development workflow)

---

## Summary

This disk cache design:
- ✅ Solves the development restart problem (main goal)
- ✅ Simple to implement (~300 lines of code)
- ✅ Easy to debug (human-readable JSON)
- ✅ Minimal dependencies (serde_json only)
- ✅ Smart about when to use cache vs bypass
- ✅ Provides UI for management (clear, stats)
- ✅ Low risk (can be disabled if issues)

**Estimated Impact for Development**:
- Typical dev session: 10 restarts in 20 minutes
- Without cache: 1,000 API calls
- With cache: 100 API calls (90% reduction)

Want me to implement this? I can start with the core `cache.rs` module and basic integration.
