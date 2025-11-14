# PR Selection and Indexing - Quick Reference Summary

## Key Findings

### Two Parallel Selection Systems

The codebase manages PR selection with two independent mechanisms that must stay in sync:

```
┌─────────────────────────────────────────────┐
│  ReposState (Legacy)                        │
├─────────────────────────────────────────────┤
│ pub prs: Vec<Pr>                            │ ← PR list
│ pub state: TableState                       │ ← Focus (cursor)
│ pub selected_prs: Vec<usize>                │ ← Multi-select indices
└─────────────────────────────────────────────┘
                    ↕ (Manual Syncing)
┌─────────────────────────────────────────────┐
│  RepoData (Modern)                          │
├─────────────────────────────────────────────┤
│ pub prs: Vec<Pr>                            │ ← PR list (duplicate)
│ pub table_state: TableState                 │ ← Focus (duplicate)
│ pub selected_prs: Vec<usize>                │ ← Indices (duplicate)
└─────────────────────────────────────────────┘
```

**Problem:** Data duplication with manual sync = fragility

---

## Index Management: Three Levels of Complexity

### Level 1: Repository Index (Stable)
- Maps to repo in `recent_repos` vector
- Rebuilt correctly when repo deleted (lines 247-257)
- Uses HashMap with index as key in `repo_data`

### Level 2: PR Row Index (Fragile)
- Position in current repo's PR list (0..M)
- Stored in `selected_prs` vector
- **NO validation** when PR list size changes
- No automatic adjustment when filtering/reloading

### Level 3: PR Number (Stable)
- Actual GitHub PR# (e.g., #123)
- Used in API calls and merge operations
- Operations correctly use PR number instead of index

---

## Six Critical Pain Points

### 1. **Stale selected_prs After Filter** 
   - Scenario: Load 20 PRs, select [5,15,18], apply filter → 8 PRs
   - Result: selected_prs still [5,15,18], rendering has no matches
   - File: `src/reducer.rs:310-329`
   - Fix: Add validation after RepoDataLoaded

### 2. **No TableState Validation on Reload**
   - TableState selection not bounds-checked after PR list changes
   - File: `src/reducer.rs:316-323`
   - Fix: Validate or reset selection when list changes

### 3. **Duplicate Sync Code**
   - 4+ locations manually copy state ↔ repo_data
   - Error-prone pattern repeats in:
     - Lines 270-281 (repo deletion)
     - Lines 304-306 (repo selection)
     - Line 415 (PR toggle)
     - main.rs 651-653 (load state)
   - Fix: Single helper function to sync all fields

### 4. **Silent Index Failures**
   - Out-of-bounds indices silently ignored with `.filter_map()` 
   - File: `src/reducer.rs:474-490`
   - Example: Rebase skips indices >= prs.len() without warning
   - Fix: Validate before operation or log discrepancies

### 5. **No Auto-Refresh After Operations**
   - Merge/rebase clear selected_prs but don't reload PR list
   - Display shows stale data until user Ctrl+R
   - File: `src/reducer.rs:454-459`
   - Fix: Generate RefreshCurrentRepo effect after operations

### 6. **Missing PR Index Bounds Validation**
   - When PR list loads from async task, old indices not validated
   - Filter changes don't rebuild selected_prs
   - File: `src/reducer.rs:310-329`
   - Fix: Filter indices before storing in selected_prs

---

## Code Fragility Examples

### Invisible Corruption (Silent Bug)
```rust
// src/main.rs:495-530 - Rendering
if repo_data.selected_prs.contains(&i) {  // i=0..2
    color = selected_bg;
}
// If selected_prs=[5,8,15] but prs.len()=3, 
// this condition never matches → indices "disappear"
```

### Forgotten Sync (Logic Bug)
```rust
// src/reducer.rs - Hypothetical bug scenario
Action::MergeComplete => {
    state.selected_prs.clear();  // ✓ Synced
    // FORGOT: must sync to repo_data!
    // if let Some(data) = state.repo_data.get_mut(&...) {
    //     data.selected_prs.clear();
    // }
}
```

### Filter Change Dead Code
```rust
// src/reducer.rs:365-367
Action::CycleFilter => {
    state.filter = state.filter.next();
    // ⚠️ No effect! Filter change doesn't trigger reload
    // Must separately call Action::RefreshCurrentRepo
}
```

---

## State Transition Pain Points

### Scenario: User Filters, Selects, Reloads
```
1. Load 20 PRs from GitHub
   → RepoDataLoaded(0, [PR1..PR20])
   → RepoData { prs: [20 items], selected_prs: [] }

2. User selects PRs at indices [5,15,18]
   → TogglePrSelection (3x)
   → RepoData { prs: [20 items], selected_prs: [5,15,18] } ✓

3. User applies filter (e.g., "Feat" only)
   → CycleFilter
   → RepoData { prs: [20 items], selected_prs: [5,15,18] } ⚠️

4. User presses Refresh
   → RefreshCurrentRepo
   → [Background task fetches filtered PRs: 8 items]
   → RepoDataLoaded(0, [PR2,PR3,PR7,PR9,PR11,PR14,PR16,PR19])
   → RepoData { prs: [8 items], selected_prs: [5,15,18] } ❌ STALE!

5. Rendering tries to show selected rows
   → if selected_prs.contains(&i) where i=0..7
   → No matches! Indices [5,15,18] exceed list length
   → User sees "selected PRs disappeared"
```

**Fix would be:** Add filter step before storing in RepoData:
```rust
data.selected_prs.retain(|&idx| idx < data.prs.len());
```

---

## Sync Locations Requiring Attention

### Where State Duplication Happens
1. **Load repo** → Sync from repo_data to state (lines 302-306)
2. **Toggle PR** → Sync from state to repo_data (line 415)
3. **Delete repo** → Rebuild repo_data indices (lines 247-257)
4. **Navigate** → Sync TableState both directions (lines 383, 401)

### Risk Factors
- If merge modifies state.prs but forgets to sync repo_data → inconsistency
- If filter changes but doesn't clear selected_prs → stale indices
- If navigation doesn't sync in both directions → UI desync

---

## Recommended Prioritized Fixes

### HIGH PRIORITY (Silent Data Corruption)
1. **Add index validation in RepoDataLoaded** (Line 329)
   ```rust
   data.selected_prs.retain(|&idx| idx < data.prs.len());
   data.table_state.select(None);  // Reset focus safely
   ```

2. **Add auto-refresh after merge** (Line 554)
   ```rust
   effects.push(Effect::RefreshCurrentRepo);
   ```

### MEDIUM PRIORITY (Hidden Bugs)
3. **Extract sync helper function** (Lines 302-306, 415, 651-653)
   ```rust
   fn sync_repo_data_to_state(state: &mut ReposState) { ... }
   ```

4. **Validate bounds on every operation** (Lines 474-490)
   ```rust
   selected_prs.retain(|&idx| idx < prs.len());
   ```

### LOW PRIORITY (Code Quality)
5. **Remove duplicate state** (Consolidate state.prs vs repo_data.prs)

---

## Files to Watch

| File | Lines | Issue |
|------|-------|-------|
| reducer.rs | 310-329 | No index validation after RepoDataLoaded |
| reducer.rs | 316-323 | No TableState validation on reload |
| reducer.rs | 247-257 | Correct repo index rebuild (reference) |
| reducer.rs | 404-423 | Selection toggle logic (reference) |
| reducer.rs | 454-459 | Merge completes without refresh |
| main.rs | 495-530 | Rendering checks selected_prs |
| main.rs | 651-653 | Sync from repo_data to state |

---

## Summary

**Root Cause:** Dual parallel state (state.rs fields + repo_data HashMap) with manual sync points creates fragility.

**Why It Works Sometimes:** 
- Simple operations (navigate, toggle) mostly sync correctly
- Complex async operations (filter, reload) have validation gaps
- Silent failures (stale indices) don't crash but corrupt display

**Why It Breaks Sometimes:**
- Filter changes aren't followed by validation
- Operations don't refresh PR list automatically
- Async reload doesn't validate inherited selected_prs
- Multiple sync paths with no central validation

**Long-term Solution:** Consolidate to single source of truth (just use repo_data), eliminate state.prs/.state/.selected_prs duplicates.

