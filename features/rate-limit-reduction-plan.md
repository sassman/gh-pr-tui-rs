# GitHub API Rate Limit Reduction Plan

## Current Situation

The application currently:
- Loads **all repositories** on startup (`LoadAllRepos`)
- Fetches PR data for every configured repository
- Refreshes repositories (potentially on a schedule or manual trigger)
- Makes multiple API calls per PR (status, comments, checks, etc.)

GitHub API rate limits:
- **Authenticated requests**: 5,000/hour
- **Search API**: 30/minute
- **GraphQL**: 5,000 points/hour

---

## 🚀 Quick Wins (High Impact, Low Effort)

### 1. **Paused/Active Repository State** ⭐ RECOMMENDED
**Effort**: Low | **Impact**: High | **Priority**: P0

Add a `paused` flag to repository configuration:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Repo {
    pub org: String,
    pub repo: String,
    pub branch: String,
    #[serde(default)]
    pub paused: bool,  // NEW: Skip loading if true
}
```

**Changes needed**:
- `state.rs`: Add `paused: bool` to `Repo` struct
- `effect.rs`: Filter out paused repos in `LoadAllRepos`
- `command_palette_integration.rs`: Add "Pause Repository" / "Resume Repository" commands
- `reducer.rs`: Add `PauseRepo(usize)` and `ResumeRepo(usize)` actions
- `.recent-repositories.json`: Persist paused state

**Benefits**:
- Skip loading paused repos on startup: **~10-50 requests saved per repo**
- Skip refresh cycles for paused repos: **ongoing savings**
- User can pause repos they don't actively monitor

**Implementation**:
```rust
// In LoadAllRepos effect
let active_repos: Vec<_> = repos
    .iter()
    .filter(|(_, repo)| !repo.paused)
    .cloned()
    .collect();
```

---

### 2. **Lazy Loading (Load on Tab Switch)** ⭐ RECOMMENDED
**Effort**: Medium | **Impact**: High | **Priority**: P0

Only load PRs for the **currently selected** repository on startup.
Load other repos when user switches to them.

**Changes needed**:
- `reducer.rs`: On `BootstrapComplete`, only load selected repo
- `reducer.rs`: On `SelectNextRepo`/`SelectPreviousRepo`, check if repo is loaded
- `state.rs`: Add `loaded: bool` flag to `RepoData`
- `effect.rs`: Create `LoadSingleRepo` effect

**Benefits**:
- Startup time: **Load 1 repo instead of N repos**
- If user has 10 repos but only checks 2-3 regularly: **70-80% reduction** in API calls

**Implementation**:
```rust
// state.rs
pub struct RepoData {
    pub prs: Vec<Pr>,
    pub table_state: TableState,
    pub selected_pr_numbers: HashSet<PrNumber>,
    pub loaded: bool,  // NEW: Track if this repo has been loaded
    pub last_loaded: Option<Instant>,  // NEW: Cache invalidation
}

// reducer.rs - SelectNextRepo
Action::SelectNextRepo => {
    // ... existing navigation logic ...

    // Check if newly selected repo needs loading
    if let Some(data) = state.repo_data.get(&state.selected_repo) {
        if !data.loaded {
            effects.push(Effect::LoadSingleRepo {
                repo_index: state.selected_repo,
                repo: repos[state.selected_repo].clone(),
            });
        }
    }
}
```

---

### 3. **Configurable Refresh Intervals**
**Effort**: Low | **Impact**: Medium | **Priority**: P1

Allow users to configure refresh intervals per-repo or globally.

**Changes needed**:
- Add `refresh_interval_minutes` to Repo config (default: 5)
- Add global setting in config file
- Track `last_refreshed` timestamp per repo
- Skip refresh if interval hasn't elapsed

**Benefits**:
- User sets 15-minute refresh instead of 5-minute: **66% reduction** in refresh calls
- Critical repos: 2 minutes, others: 30 minutes

**Implementation**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub org: String,
    pub repo: String,
    pub branch: String,
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_minutes: u64,  // NEW
}

fn default_refresh_interval() -> u64 { 5 }

// RepoData
pub struct RepoData {
    // ... existing fields ...
    pub last_refreshed: Option<Instant>,
}

// Before refresh
let should_refresh = data.last_refreshed
    .map(|t| t.elapsed().as_secs() >= repo.refresh_interval_minutes * 60)
    .unwrap_or(true);
```

---

## 🔧 Medium Effort Improvements

### 4. **Conditional Requests with ETags**
**Effort**: Medium | **Impact**: High | **Priority**: P1

Use `If-None-Match` headers with ETags to avoid unnecessary data transfer.
GitHub doesn't count **304 Not Modified** responses against rate limits!

**Changes needed**:
- Store ETag per repo/PR in `RepoData`
- Add ETag header to API requests
- Handle 304 responses (data unchanged)
- Update cache only on 200 responses

**Benefits**:
- **304 responses don't count against rate limit**
- Reduce bandwidth usage
- If 70% of refreshes have no changes: **effective 3.3x rate limit increase**

**Implementation**:
```rust
// state.rs
pub struct RepoData {
    // ... existing fields ...
    pub etag: Option<String>,  // NEW: Store ETag from last fetch
}

// In gh client wrapper
async fn fetch_prs_with_etag(
    octocrab: &Octocrab,
    repo: &Repo,
    etag: Option<&str>,
) -> Result<(Vec<Pr>, Option<String>), Error> {
    let mut builder = octocrab.get(...);

    if let Some(etag) = etag {
        builder = builder.header("If-None-Match", etag);
    }

    let response = builder.send().await?;

    if response.status() == 304 {
        return Ok((vec![], None));  // No changes, return empty
    }

    let new_etag = response.headers()
        .get("ETag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let prs = response.json().await?;
    Ok((prs, new_etag))
}
```

---

### 5. **Background vs Foreground Priority**
**Effort**: Medium | **Impact**: Medium | **Priority**: P2

Only auto-refresh the **currently visible** repository.
Other repos refresh manually or on a longer interval.

**Changes needed**:
- Track "active" repo (currently selected)
- Implement refresh scheduler that prioritizes active repo
- Background repos: refresh only on manual trigger or every 30+ minutes

**Benefits**:
- If user has 10 repos but actively views 1: **90% reduction** in refresh calls
- Better UX (active repo stays fresh)

---

### 6. **Cache to Disk Between Runs**
**Effort**: Medium | **Impact**: Medium | **Priority**: P2

Persist fetched PR data to disk, load from cache on startup.

**Changes needed**:
- Serialize `RepoData` to disk (`.cache/repo-{org}-{repo}.json`)
- Add TTL/expiry time to cache
- On startup: load from cache if fresh, fetch if stale
- Use ETags for cache validation

**Benefits**:
- **Zero API calls** on startup if cache is fresh (< 5 minutes old)
- Instant app startup
- Survives restarts without re-fetching

**Implementation**:
```rust
// On startup
match load_cached_repo_data(&repo) {
    Some(cached) if !cached.is_stale() => {
        // Use cached data, validate with ETag in background
        state.repo_data.insert(index, cached);
    }
    _ => {
        // Cache miss or stale, fetch from API
        effects.push(Effect::LoadSingleRepo { ... });
    }
}
```

---

### 7. **Reduce Optional Data Fetching**
**Effort**: Low-Medium | **Impact**: Medium | **Priority**: P2

Make expensive optional features opt-in or lazy-loaded:

**Current behavior** (expensive):
- Fetch mergeable status for all PRs
- Fetch comment counts for all PRs
- Fetch check runs for all PRs
- Monitor build status continuously

**Optimization**:
- **Lazy load**: Only fetch when user views PR details
- **Opt-in**: Config flag `fetch_comment_counts: false`
- **Batch**: Fetch multiple PRs' data in one GraphQL query

**Benefits**:
- If comment counts use 1 request per PR, disabling saves **N requests per refresh**
- GraphQL batching: 10 separate requests → 1 batched request

---

## 🏗️ Long-term Enhancements

### 8. **GraphQL Migration for Batch Fetching**
**Effort**: High | **Impact**: High | **Priority**: P3

Migrate from REST API to GraphQL for batch operations.

**Benefits**:
- Fetch PRs + statuses + comments in **one request** instead of 10+
- More efficient use of rate limit (points vs requests)
- Example: Current 50 REST calls → 1 GraphQL query (costs ~50 points but one request)

**Example**:
```graphql
query {
  repository(owner: "org", name: "repo") {
    pullRequests(first: 50, states: OPEN) {
      nodes {
        number
        title
        mergeable
        comments { totalCount }
        commits(last: 1) {
          nodes {
            commit {
              statusCheckRollup { state }
            }
          }
        }
      }
    }
  }
}
```

---

### 9. **Webhook Integration (Advanced)**
**Effort**: Very High | **Impact**: Very High | **Priority**: P4

Use GitHub webhooks to get **push notifications** instead of polling.

**Benefits**:
- **Zero polling** - only fetch when changes occur
- Near-instant updates
- Minimal rate limit usage

**Challenges**:
- Requires webhook server (ngrok for local dev)
- Complex setup
- Not suitable for CLI/desktop app without backend

---

### 10. **Smart Refresh Based on Activity**
**Effort**: Medium | **Impact**: Medium | **Priority**: P3

Only refresh repos that are likely to have changed:

**Heuristics**:
- Skip refresh if last refresh showed "no PRs"
- Skip refresh if repo hasn't been viewed in 24 hours
- Increase refresh interval for inactive PRs (no updates in 7 days)
- Track "hot" repos (frequent changes) vs "cold" repos

---

## 📋 Recommended Implementation Order

### Phase 1: Immediate Relief (Week 1)
1. ✅ **Paused/Active Repository State** - Easiest, biggest impact
2. ✅ **Lazy Loading on Tab Switch** - Reduces startup load dramatically
3. ✅ **Configurable Refresh Intervals** - Let users control rate limit usage

### Phase 2: Optimization (Week 2-3)
4. ✅ **ETags for Conditional Requests** - Free rate limit boost
5. ✅ **Disk Cache Between Runs** - Eliminate startup API calls
6. ✅ **Background Priority** - Only auto-refresh visible repo

### Phase 3: Advanced (Month 2+)
7. ✅ **Reduce Optional Data** - Make expensive features opt-in
8. ✅ **GraphQL Migration** - Batch operations
9. ✅ **Smart Refresh** - Intelligent refresh scheduling

---

## 💡 Additional Tips

### Monitor Rate Limit Usage
Add a status indicator showing:
- Remaining rate limit: `X-RateLimit-Remaining` header
- Reset time: `X-RateLimit-Reset` header
- Visual warning when < 1000 remaining

```rust
// Parse from GitHub API response headers
pub struct RateLimitInfo {
    pub remaining: u32,
    pub limit: u32,
    pub reset_time: DateTime<Utc>,
}
```

### Graceful Degradation
When rate limit is low:
- Disable auto-refresh
- Show warning banner
- Suggest pausing non-critical repos
- Automatically increase refresh intervals

### User Education
Add help text explaining:
- How many requests different operations cost
- How to pause repos
- Benefits of longer refresh intervals
- ETags and caching benefits

---

## 📊 Expected Impact

**Current state** (example with 10 repos):
- Startup: 10 repos × 10 requests = **100 requests**
- Refresh (every 5 min): 10 repos × 10 requests = **100 requests/5 min** = 1,200/hour
- **Total**: ~1,300 requests/hour → **Will hit 5K limit in ~4 hours**

**With Phase 1 optimizations**:
- Startup: 1 repo × 10 requests = **10 requests** (90% reduction)
- 8 repos paused, 2 active
- Refresh (15 min interval): 2 repos × 10 requests = **20 requests/15 min** = 80/hour
- **Total**: ~90 requests/hour → **Can run for 55+ hours on 5K limit**

**With Phase 2 (+ ETags)**:
- 70% of refreshes = 304 (not counted)
- Effective refresh cost: 30% of 80/hour = **24/hour**
- Disk cache eliminates startup cost on restarts
- **Total**: ~24 requests/hour → **Can run for 208+ hours on 5K limit**

---

## 🎯 My Recommendation: Start Here

**Implement these 3 features first**:

1. **Paused Repositories**
   - Add UI command: "Pause Repository" / "Resume Repository"
   - Filter paused repos in `LoadAllRepos`
   - Persist to `.recent-repositories.json`

2. **Lazy Loading**
   - Only load selected repo on startup
   - Load others on tab switch
   - Show "Loading..." indicator when switching to unloaded repo

3. **Refresh Interval Config**
   - Add `refresh_interval_minutes` to repo config
   - Default: 15 minutes (up from implied 5)
   - Show last refresh time in UI

**These 3 changes alone will reduce API usage by 80-90% for most users.**

Want me to implement any of these? I'd suggest starting with **Paused Repositories** as it's the simplest and most user-friendly.
