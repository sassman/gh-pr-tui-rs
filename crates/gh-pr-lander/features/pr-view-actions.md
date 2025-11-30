# Pull Request View Actions Implementation Plan

This document outlines the implementation plan for all PR-related actions in the main PR view,
based on the functionality in `gh-pr-tui`.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Action Flow                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User Input (Key)                                                            │
│       │                                                                      │
│       ▼                                                                      │
│  KeyboardMiddleware ──► Translates to semantic action based on capabilities │
│       │                                                                      │
│       ▼                                                                      │
│  Domain Middleware ───► Handles side effects (API calls, file I/O)          │
│  (e.g., PullRequestMiddleware, GitHubMiddleware)                            │
│       │                                                                      │
│       ▼                                                                      │
│  Reducer ─────────────► Pure state transformation                           │
│       │                                                                      │
│       ▼                                                                      │
│  New State ───────────► Triggers UI re-render                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Responsibility Separation

| Layer | Responsibility | Examples |
|-------|---------------|----------|
| **KeyboardMiddleware** | Translate raw keys to semantic actions | `'j'` → `PrNavigateNext` |
| **Domain Middleware** | Side effects, API calls, async operations | Fetch PRs, merge, rebase |
| **Reducer** | Pure state transformation | Update selected PR index, store PR data |

---

## Phase 1: PR Selection & Navigation (Priority: High)

### Already Implemented
- [x] `PrNavigateNext` - Move cursor down in PR table
- [x] `PrNavigatePrevious` - Move cursor up in PR table
- [x] `PrLoadStart(repo_idx)` - Begin loading PRs
- [x] `PrLoaded(repo_idx, prs)` - PRs loaded successfully
- [x] `PrLoadError(repo_idx, error)` - PR loading failed
- [x] `PrRefresh` - Force refresh current repository

### To Implement

#### 1.1 PR Multi-Selection
Toggle selection for bulk operations (merge, approve, close).

**Actions:**
```rust
/// Toggle selection of current PR
PrToggleSelection,
/// Select all PRs in current repository
PrSelectAll,
/// Deselect all PRs
PrDeselectAll,
/// Clear selection (alias for DeselectAll)
PrClearSelection,
```

**Reducer (`reducers/pr.rs`):**
- `PrToggleSelection`: Toggle current PR in `selected_pr_numbers: HashSet<usize>`
- `PrSelectAll`: Add all PR numbers to selection
- `PrDeselectAll`: Clear selection set

**State Changes:**
```rust
// In MainViewState or per-repository state
pub struct RepositoryPrState {
    pub prs: Vec<Pr>,
    pub selected_index: usize,           // Cursor position
    pub selected_pr_numbers: HashSet<usize>, // Multi-selection
    pub loading: bool,
    pub error: Option<String>,
}
```

---

## Phase 2: PR Operations (Priority: High)

### 2.1 Open PR in Browser

**Actions:**
```rust
/// Open current PR in default browser
PrOpenInBrowser,
```

**Middleware (`middleware/browser.rs` or extend `pull_request.rs`):**
- Get current PR's `html_url`
- Call `open::that(url)` or platform-specific browser open

**Reducer:** No state change needed.

---

### 2.2 Merge PR

**Actions:**
```rust
/// Request to merge selected PRs (or cursor PR if none selected)
PrMergeRequest,
/// Merge operation started for a PR
PrMergeStart(usize, usize),  // repo_idx, pr_number
/// Merge completed successfully
PrMergeSuccess(usize, usize),  // repo_idx, pr_number
/// Merge failed
PrMergeError(usize, usize, String),  // repo_idx, pr_number, error
```

**Middleware (`middleware/github.rs` - new):**
```rust
Action::PrMergeRequest => {
    // Get target PRs (selected or cursor)
    let target_prs = get_target_prs(state);
    for pr in target_prs {
        dispatcher.dispatch(Action::PrMergeStart(repo_idx, pr.number));
        // Spawn async task to call GitHub merge API
        // On success: dispatch PrMergeSuccess
        // On error: dispatch PrMergeError
    }
}
```

**Reducer:**
- `PrMergeStart`: Set PR status to "merging"
- `PrMergeSuccess`: Remove PR from list, clear from selection
- `PrMergeError`: Set error state on PR

**API Call:**
```rust
octocrab.pulls(owner, repo).merge(pr_number).send().await
```

---

### 2.3 Rebase PR

**Actions:**
```rust
/// Request to rebase selected PRs
PrRebaseRequest,
/// Rebase operation started
PrRebaseStart(usize, usize),  // repo_idx, pr_number
/// Rebase completed successfully
PrRebaseSuccess(usize, usize),
/// Rebase failed
PrRebaseError(usize, usize, String),
```

**Middleware (`middleware/github.rs`):**
- Call GitHub update branch API
- This updates the PR's head branch with latest from base

**API Call:**
```rust
// PUT /repos/{owner}/{repo}/pulls/{pull_number}/update-branch
octocrab.put(format!("/repos/{}/{}/pulls/{}/update-branch", owner, repo, pr_number), None::<&()>).await
```

---

### 2.4 Approve PR

**Actions:**
```rust
/// Request to approve selected PRs
PrApproveRequest,
/// Approval started
PrApproveStart(usize, usize),
/// Approval completed
PrApproveSuccess(usize, usize),
/// Approval failed
PrApproveError(usize, usize, String),
```

**Middleware (`middleware/github.rs`):**
- Create a review with "APPROVE" event

**API Call:**
```rust
octocrab.pulls(owner, repo)
    .create_review(pr_number)
    .event(ReviewEvent::Approve)
    .send().await
```

---

### 2.5 Close PR

**Actions:**
```rust
/// Show close PR confirmation popup
PrCloseShowPopup,
/// Hide close PR popup
PrCloseHidePopup,
/// Confirm close with optional comment
PrCloseConfirm(Option<String>),  // Optional close comment
/// Close operation started
PrCloseStart(usize, usize),
/// Close completed
PrCloseSuccess(usize, usize),
/// Close failed
PrCloseError(usize, usize, String),
```

**Middleware (`middleware/github.rs`):**
- Update PR state to "closed"

**API Call:**
```rust
octocrab.pulls(owner, repo).update(pr_number).state(State::Closed).send().await
```

---

## Phase 3: Build Status & CI (Priority: Medium)

### 3.1 Check Build Status

**Actions:**
```rust
/// Check build status for a PR
PrCheckBuildStatus(usize, usize, String),  // repo_idx, pr_number, head_sha
/// Build status updated
PrBuildStatusUpdated(usize, usize, BuildStatus),
```

**Middleware (`middleware/github.rs`):**
- Fetch check runs and commit status
- Combine into overall status

**API Calls:**
```rust
// Check runs (GitHub Actions)
client.fetch_check_runs(owner, repo, sha).await
// Commit status (external CI)
client.fetch_commit_status(owner, repo, sha).await
```

**State:**
```rust
pub enum BuildStatus {
    Pending,
    InProgress,
    Success,
    Failure,
    Unknown,
}
```

---

### 3.2 Rerun Failed Jobs

**Actions:**
```rust
/// Request to rerun failed CI jobs
PrRerunFailedJobs,
/// Rerun started
PrRerunJobsStart(usize, usize),
/// Rerun completed
PrRerunJobsSuccess(usize, usize),
/// Rerun failed
PrRerunJobsError(usize, usize, String),
```

**Middleware (`middleware/github.rs`):**
- Find failed workflow runs
- Trigger re-run via API

**API Call:**
```rust
// POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun-failed-jobs
octocrab.post(format!("/repos/{}/{}/actions/runs/{}/rerun-failed-jobs", owner, repo, run_id), None::<&()>).await
```

---

### 3.3 Open Build Logs

**Actions:**
```rust
/// Open build logs view for current PR
PrOpenBuildLogs,
/// Build logs loaded
PrBuildLogsLoaded(Vec<JobLog>),
/// Build logs load error
PrBuildLogsError(String),
/// Close build logs view
PrCloseBuildLogs,
```

**Middleware (`middleware/github.rs`):**
- Fetch workflow runs for the PR
- Fetch job logs for each run
- Parse logs using `gh-actions-log-parser`

**View:**
- Push `BuildLogsView` onto view stack
- Tree navigation for jobs/steps
- Log content viewer with scroll

---

## Phase 4: IDE Integration (Priority: Low)

### 4.1 Open in IDE

**Actions:**
```rust
/// Open current PR in configured IDE
PrOpenInIDE,
/// IDE open completed
PrOpenInIDESuccess,
/// IDE open failed
PrOpenInIDEError(String),
```

**Middleware (`middleware/ide.rs` - new):**
- Determine IDE from config/environment
- Clone/checkout PR branch if needed
- Open IDE with project path

---

## Phase 5: Merge Bot (Priority: Medium)

Automated merge queue that monitors PRs and merges when ready.

### 5.1 Merge Bot Actions

**Actions:**
```rust
/// Start merge bot for selected PRs
MergeBotStart,
/// Initialize merge bot with PR data
MergeBotInit(Vec<(usize, usize)>),  // [(repo_idx, pr_number)]
/// Periodic tick for merge bot processing
MergeBotTick,
/// Add PR to merge queue
MergeBotAddToQueue(usize, usize),
/// Remove PR from queue
MergeBotRemoveFromQueue(usize, usize),
/// PR status check
MergeBotStatusCheck(usize, usize),
/// Merge bot stopped
MergeBotStop,
```

**Middleware (`middleware/merge_bot.rs` - new):**
- Maintain queue of PRs to merge
- Periodically check CI status
- Merge when all checks pass
- Handle conflicts and failures

**State:**
```rust
pub struct MergeBotState {
    pub active: bool,
    pub queue: Vec<MergeBotEntry>,
    pub current: Option<MergeBotEntry>,
}

pub struct MergeBotEntry {
    pub repo_idx: usize,
    pub pr_number: usize,
    pub status: MergeBotStatus,
    pub last_check: Instant,
}

pub enum MergeBotStatus {
    Queued,
    CheckingCI,
    WaitingForCI,
    ReadyToMerge,
    Merging,
    Merged,
    Failed(String),
}
```

---

## Phase 6: Filter & Search (Priority: Medium)

### 6.1 PR Filtering

**Actions:**
```rust
/// Cycle through filter presets
PrCycleFilter,
/// Set specific filter
PrSetFilter(PrFilter),
```

**State:**
```rust
pub enum PrFilter {
    All,
    ReadyToMerge,
    NeedsRebase,
    BuildFailed,
    MyPRs,
    Custom(String),  // Title/author search
}
```

**Reducer:**
- Update `current_filter` in state
- View model filters PRs for display

---

## Implementation Order

### Milestone 1: Core PR Operations
1. PR Multi-Selection (`PrToggleSelection`, `PrSelectAll`, `PrDeselectAll`)
2. Open in Browser (`PrOpenInBrowser`)
3. Build Status Checking (`PrCheckBuildStatus`, `PrBuildStatusUpdated`)

### Milestone 2: GitHub Operations
4. Merge PR (`PrMergeRequest` → `PrMergeSuccess/Error`)
5. Rebase PR (`PrRebaseRequest` → `PrRebaseSuccess/Error`)
6. Approve PR (`PrApproveRequest` → `PrApproveSuccess/Error`)

### Milestone 3: CI Integration
7. Rerun Failed Jobs (`PrRerunFailedJobs`)
8. Build Logs View (`PrOpenBuildLogs`, log parsing)

### Milestone 4: Automation
9. Merge Bot (queue management, auto-merge)

### Milestone 5: Extras
10. Close PR with popup
11. PR Filtering
12. IDE Integration

---

## New Middleware Structure

After implementing all features:

```
middleware/
├── mod.rs
├── bootstrap.rs      # App startup, tick thread
├── keyboard.rs       # Key → semantic action translation
├── command_palette.rs
├── repository.rs     # Repository loading, add repo form
├── pull_request.rs   # PR loading, caching
├── github.rs         # NEW: GitHub API operations (merge, rebase, approve, CI)
├── merge_bot.rs      # NEW: Automated merge queue
├── browser.rs        # NEW: Open URLs in browser
├── ide.rs            # NEW: IDE integration (optional)
└── logging.rs
```

---

## Extending `gh-client` Crate

The `gh-client` crate should be extended with these methods on `GitHubClient`:

```rust
#[async_trait]
pub trait GitHubClient: Send + Sync + std::fmt::Debug {
    // Existing
    async fn fetch_pull_requests(...) -> Result<Vec<PullRequest>>;
    async fn fetch_check_runs(...) -> Result<Vec<CheckRun>>;
    async fn fetch_commit_status(...) -> Result<CheckStatus>;

    // New - PR Operations
    async fn merge_pull_request(
        &self, owner: &str, repo: &str, pr_number: u64,
        merge_method: MergeMethod,
    ) -> Result<MergeResult>;

    async fn update_pull_request_branch(
        &self, owner: &str, repo: &str, pr_number: u64,
    ) -> Result<()>;

    async fn create_review(
        &self, owner: &str, repo: &str, pr_number: u64,
        event: ReviewEvent, body: Option<&str>,
    ) -> Result<()>;

    async fn close_pull_request(
        &self, owner: &str, repo: &str, pr_number: u64,
    ) -> Result<()>;

    // New - CI Operations
    async fn rerun_failed_jobs(
        &self, owner: &str, repo: &str, run_id: u64,
    ) -> Result<()>;

    async fn fetch_workflow_runs(
        &self, owner: &str, repo: &str, head_sha: &str,
    ) -> Result<Vec<WorkflowRun>>;

    async fn fetch_job_logs(
        &self, owner: &str, repo: &str, job_id: u64,
    ) -> Result<String>;
}
```

---

## Notes

- All GitHub API operations go through `gh-client` for caching and abstraction
- Middleware handles async operations and dispatches result actions
- Reducer only does pure state transformations
- View models transform state for rendering (filtering, sorting, formatting)
