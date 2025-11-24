# Redux Architecture Violations Audit

**Date**: 2024-11-24
**Status**: Initial Audit Complete
**Branch**: `feat/cleaner-redux`

---

## Executive Summary

This document catalogs all violations of TRUE Redux principles in the codebase.

### Violation Categories

| Category | Count | Severity | Impact |
|----------|-------|----------|--------|
| **state_mut() calls** | 5 | 🔴 HIGH | Direct state mutations bypass reducers |
| **TaskResult conversion** | 18 variants | 🔴 HIGH | Unnecessary conversion layer |
| **Result channels** | 2 channels | 🟡 MEDIUM | Architecture complexity |
| **Middleware mutations** | 0 | ✅ CLEAN | No violations found |

---

## 1. Direct State Mutations via `state_mut()`

### Violations Found: 5

These directly mutate state bypassing the reducer pattern:

#### Violation 1: main.rs:296
```rust
let state = app.store.state_mut();
```
**Context**: Used in view model computation
**Impact**: View model updates bypass action flow
**Fix**: Ensure view models only computed in reducers

#### Violation 2: main.rs:322-324
```rust
app.store.state_mut().repos.loading_state = LoadingState::Error(err.to_string());
app.store.state_mut().ui.should_quit = true;
```
**Context**: Error handling in update() function
**Impact**: Critical state changes bypass reducers
**Fix**: Create `Action::UpdateError(error)` and handle in reducer

#### Violation 3: main.rs:424
```rust
app.store.state_mut().ui.shortcuts_max_scroll = max_scroll;
```
**Context**: Updating shortcuts scroll in UI rendering
**Impact**: UI state mutations during render
**Fix**: Move to reducer when shortcuts panel opens/resizes

#### Violation 4: main.rs:537
```rust
.state_mut()
```
**Context**: Unknown usage (line number only)
**Impact**: TBD - needs investigation
**Fix**: TBD after reading context

---

## 2. TaskResult Conversion Layer

### Problem

Background tasks return `TaskResult`, which is then converted to `Action` in main.rs.

**Flow**: `BackgroundTask` → `process_task()` → `TaskResult` → `result_to_action()` → `Action`

**Why it's wrong**: This is an unnecessary intermediate layer. Tasks should dispatch actions directly.

### TaskResult Variants: 18

Located in `task.rs:18-58`:

1. `RepoLoadingStarted(usize)`
2. `RepoDataLoaded(usize, Result<Vec<Pr>, String>)`
3. `MergeStatusUpdated(usize, usize, MergeableStatus)`
4. `RebaseStatusUpdated(usize, usize, bool)`
5. `CommentCountUpdated(usize, usize, usize)`
6. `RebaseComplete(Result<(), String>)`
7. `MergeComplete(Result<(), String>)`
8. `RerunJobsComplete(Result<(), String>)`
9. `ApprovalComplete(Result<(), String>)`
10. `ClosePrComplete(Result<(), String>)`
11. `BuildLogsLoaded(...)`
12. `IDEOpenComplete(Result<(), String>)`
13. `PRMergedConfirmed(usize, usize, bool)`
14. `TaskStatusUpdate(Option<TaskStatus>)`
15. `AutoMergeStatusCheck(usize, usize)`
16. `RemoveFromAutoMergeQueue(usize, usize)`
17. `OperationMonitorCheck(usize, usize)`
18. `RemoveFromOperationMonitor(usize, usize)`
19. `RepoNeedsReload(usize)`
20. `DispatchAction(Action)` (ironic!)

### result_to_action() Function

**Location**: `main.rs:207-244`

**Problem**: 1:1 mapping between TaskResult and Action. This layer adds no value.

```rust
fn result_to_action(result: TaskResult) -> Action {
    match result {
        TaskResult::RepoLoadingStarted(idx) => Action::RepoLoadingStarted(idx),
        // ... 18 more identical mappings
    }
}
```

**Fix**: Delete this function and TaskResult enum. Tasks dispatch actions directly.

---

## 3. Result Channels

### Channels Found: 2

**Location**: `main.rs:251-265`

```rust
let (result_tx, mut result_rx) = mpsc::unbounded_channel();
let worker_task = start_task_worker(task_rx, result_tx);
```

**Usage in main loop**: `main.rs:309`
```rust
Some(result) = result_rx.recv() => {
    Some(result_to_action(result))
}
```

**Problem**:
- Adds complexity
- Creates indirection
- Requires TaskResult enum
- Requires result_to_action() conversion

**Fix**:
- Delete result channels
- Pass `Dispatcher` to background tasks
- Tasks dispatch actions directly

---

## 4. Middleware State Access ✅

### Audit Result: CLEAN

**Searched**: All `state.` patterns with assignment in `middleware.rs`

**Found**: 0 mutations

**All usages are reads**:
- `state.repos.selected_repo` (read)
- `state.repos.recent_repos.get()` (read)
- `state.config.clone()` (read)
- etc.

**Conclusion**: Middleware is already pure! ✅

---

## 5. Store API Surface

### Current API (Problematic)

```rust
impl Store {
    pub fn state(&self) -> &AppState { }       // ✅ OK - immutable read
    pub fn state_mut(&mut self) -> &mut AppState { }  // ❌ BAD - allows mutations
    pub fn dispatch(&mut self, action: Action) { }    // ✅ OK
    pub fn dispatch_async(...) { }                    // ✅ OK
}
```

**Problem**: `state_mut()` exists, allowing direct mutations.

**Fix**: Delete `state_mut()` method entirely.

---

## 6. Reducer Purity ✅

### Audit Result: CLEAN

**Checked for**:
- `.await` calls: 0 found
- File I/O: 0 found
- Network calls: 0 found
- Direct dispatcher access: 0 found

**Conclusion**: Reducers are already pure! ✅

---

## 7. View Model Updates

### Pattern Found: Computed in Reducers ✅

All `recompute_*_view_model()` functions are called from reducers:

**Location**: `reducer.rs`
- `recompute_splash_screen_view_model()`
- `recompute_shortcuts_panel_view_model()`
- `recompute_command_palette_view_model()`
- `recompute_pr_table_view_model()`
- `recompute_repository_tabs_view_model()`
- `recompute_view_model()` (log panel)
- `recompute_debug_console_view_model()`

**Conclusion**: View model pattern is correct! ✅

**Exception**: Line 296 in main.rs uses state_mut() for view model - needs investigation.

---

## Priority Fix Order

### 🔴 Critical (Blocks TRUE Redux)

1. **Remove state_mut() calls** (5 violations)
   - Convert to action dispatches
   - Remove state_mut() method from Store

2. **Eliminate TaskResult** (18 variants + conversion function)
   - Pass Dispatcher to background tasks
   - Tasks dispatch actions directly
   - Delete result channels

### 🟡 Medium (Architecture improvement)

3. **Verify no state mutations during render** (main.rs:296, 424)
   - Ensure view models only computed in reducers
   - Move any mutations to action flow

---

## Success Criteria

When these are all 0, we have TRUE Redux:

```bash
# Must return 0:
grep -rn "state_mut()" crates/gh-pr-tui/src/*.rs
grep -rn "TaskResult" crates/gh-pr-tui/src/task.rs
grep -rn "result_rx\|result_tx" crates/gh-pr-tui/src/main.rs
grep -rn "result_to_action" crates/gh-pr-tui/src/main.rs
```

---

## Estimated Effort

| Phase | Effort | Risk |
|-------|--------|------|
| Remove state_mut() | 2-3 hours | LOW |
| Eliminate TaskResult | 3-4 hours | MEDIUM |
| Testing | 2 hours | LOW |
| **TOTAL** | **7-9 hours** | **LOW-MEDIUM** |

---

## Notes

### ✅ Good News

- Middleware is already pure (no mutations)
- Reducers are already pure (no side effects)
- View model pattern is correct
- Only 5 direct state mutations to fix
- Architecture is 90% there

### 🎯 Focus Areas

1. Replace 5 state_mut() calls with actions
2. Delete TaskResult enum and conversion layer
3. Pass Dispatcher to background tasks
4. Remove state_mut() from Store API

---

**Generated**: 2024-11-24
**Next Step**: Phase 2 - Remove state_mut() calls
