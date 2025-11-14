# PR Selection and Indexing Analysis

## Overview

This codebase uses a table-based UI (Ratatui) to display GitHub pull requests with row selection capability. The system manages two parallel index tracking mechanisms:
1. **TableState**: Ratatui widget state tracking the cursor/focused row
2. **selected_prs**: Vector of usize indices for bulk multi-selection

### State Location
- **Primary**: `ReposState` in `src/state.rs`
- **Per-repo storage**: `RepoData` in `src/state.rs` (HashMap keyed by repo index)
- **Legacy fields**: Duplicate copies in `ReposState` for backward compatibility

---

## 1. Where `selected_prs` is Used and Modified

### Definition
```rust
// src/state.rs lines 70-81
pub struct ReposState {
    pub prs: Vec<Pr>,           // The actual PR data
    pub state: TableState,      // Ratatui table cursor position
    pub selected_prs: Vec<usize>, // Indices of selected rows
    ...
}

pub struct RepoData {
    pub prs: Vec<Pr>,
    pub table_state: TableState,
    pub selected_prs: Vec<usize>,  // Per-repo storage
    ...
}
```

### Usage Locations

**1. Reading (containing checks):**
- `src/reducer.rs:406` - Check if PR index is in selected list
- `src/main.rs:501` - Render highlight color for selected rows
- `src/main.rs:1310` - Toggle selection status

**2. Writing (adding/removing selections):**
- `src/reducer.rs:404-423` - `Action::TogglePrSelection`
  ```rust
  if state.selected_prs.contains(&selected) {
      state.selected_prs.retain(|&i| i != selected);  // Remove
  } else {
      state.selected_prs.push(selected);              // Add
  }
  state.selected_prs.sort_unstable();
  ```

**3. Clearing:**
- `src/reducer.rs:319` - When PR list is empty
- `src/reducer.rs:456-459` - After successful merge
- `src/main.rs:1465` - When selecting a repo

**4. Syncing (keeping replicas in sync):**
- `src/reducer.rs:270-281` - After repo deletion
- `src/reducer.rs:304-306` - After repo selection
- `src/reducer.rs:415` - After toggling PR selection
- `src/main.rs:651-653` - Load repo state

---

## 2. How TableState Selection Works

### Ratatui TableState Basics
```rust
// src/state.rs - TableState is from ratatui crate
pub state: TableState,  // Holds Option<usize> for selected row

// Setter/Getter methods:
data.table_state.select(Some(0))    // Set selection to row 0
data.table_state.select(None)       // Clear selection
data.table_state.selected()         // Returns Option<usize>
```

### Selection Update Flow

**User navigates with arrow keys:**
1. `src/reducer.rs:368-385` - `Action::NavigateToNextPr`
   ```rust
   let i = match state.state.selected() {
       Some(i) => (i + 1) % state.prs.len(),  // Circular nav
       None => 0,
   };
   state.state.select(Some(i));  // Update TableState
   
   // CRITICAL: Must sync to repo_data
   if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
       data.table_state.select(Some(i));
   }
   ```

2. Similar for `Action::NavigateToPreviousPr` (lines 386-402)

### Selection Rendering
```rust
// src/main.rs:495-530
let rows = repo_data.prs.iter().enumerate().map(|(i, item)| {
    // Apply selection color if this row is selected
    let color = if repo_data.selected_prs.contains(&i) {
        app.store.state().theme.selected_bg
    } else {
        color
    };
    ...
});

// Render table with stateful widget
let table_state = &mut app.get_current_repo_data_mut().table_state;
f.render_stateful_widget(table, table_area, table_state);
```

**Two parallel concepts:**
- **TableState::selected()** = Currently focused row (cursor position)
- **selected_prs vector** = Bulk-selected rows (like checkbox selection)

---

## 3. What Happens When PRs are Filtered, Reloaded, or Removed

### Scenario A: Loading PRs for a Repository

**Code:** `src/reducer.rs:310-329` - `Action::RepoDataLoaded`

```rust
Action::RepoDataLoaded(repo_index, Ok(prs)) => {
    let data = state.repo_data.entry(*repo_index).or_default();
    data.prs = prs.clone();
    data.loading_state = LoadingState::Loaded;

    // CRITICAL: Reclaim table selection when list changes
    if data.prs.is_empty() {
        // Clear selection when no PRs
        data.table_state.select(None);
        data.selected_prs.clear();
    } else if data.table_state.selected().is_none() {
        // Auto-select first row if nothing selected
        data.table_state.select(Some(0));
    }
    // ⚠️  FRAGILITY: Does NOT validate selected_prs indices against new PR list
    // If old indices exceed new list length, they become stale!
}
```

**Pain Point:** 
- Old `selected_prs` indices are NOT validated against new PR count
- Example: Had 10 PRs with indices [5,8,9] selected, now load 3 PRs
  - `selected_prs` still contains [5,8,9] 
  - Rendering checks `if selected_prs.contains(&i)` which never matches i=[0,1,2]
  - Selected rows visually disappear but indices remain in vector

### Scenario B: Cycling/Changing Filters

**Code:** `src/reducer.rs:365-367` - `Action::CycleFilter`

```rust
Action::CycleFilter => {
    state.filter = state.filter.next();
    // ⚠️  No effect generated! Filter change alone doesn't reload PRs
}
```

**Issue:** Filter changes don't trigger reload - requires separate `Action::RefreshCurrentRepo`

### Scenario C: Refreshing Current Repository

**Code:** `src/reducer.rs:461-470` - `Action::RefreshCurrentRepo`

```rust
Action::RefreshCurrentRepo => {
    if let Some(repo) = state.recent_repos.get(state.selected_repo).cloned() {
        effects.push(Effect::LoadSingleRepo {
            repo_index: state.selected_repo,
            repo,
            filter: state.filter.clone(),
        });
    }
    // ⚠️  No immediate clearing of selected_prs
    // New PR list loads via RepoDataLoaded action (async)
}
```

**Flow:**
1. User presses Ctrl+R
2. Effect spawns background task to load PRs
3. Background task calls GitHub API
4. Task dispatches `Action::RepoDataLoaded` (async)
5. At that point, indices are checked (but not validated!)

### Scenario D: Deleting Repository

**Code:** `src/reducer.rs:236-296` - `Action::DeleteCurrentRepo`

```rust
Action::DeleteCurrentRepo => {
    let selected_idx = state.selected_repo;
    state.recent_repos.remove(selected_idx);
    state.repo_data.remove(&selected_idx);

    // CRITICAL INDEX REBUILD:
    let mut new_repo_data = std::collections::HashMap::new();
    for (old_idx, data) in state.repo_data.iter() {
        let new_idx = if *old_idx > selected_idx {
            old_idx - 1  // Shift down indices above deleted repo
        } else {
            *old_idx
        };
        new_repo_data.insert(new_idx, data.clone());
    }
    state.repo_data = new_repo_data;

    // Then sync legacy fields
    if let Some(data) = state.repo_data.get(&state.selected_repo) {
        state.prs = data.prs.clone();
        state.selected_prs = data.selected_prs.clone();  // Copy from repo_data
        ...
    }
}
```

**This is CORRECT:** Repository indices are updated, and selected_prs flows from repo_data.
**BUT:** selected_prs contains PR row indices, not repo indices - no adjustment needed here.

---

## 4. Index Adjustment and Rebuilding Logic

### Repository-Level Index Rebuilding (Lines 247-257)

```rust
// Rebuild repo_data with updated indices
let mut new_repo_data = std::collections::HashMap::new();
for (old_idx, data) in state.repo_data.iter() {
    let new_idx = if *old_idx > selected_idx {
        old_idx - 1
    } else {
        *old_idx
    };
    new_repo_data.insert(new_idx, data.clone());
}
state.repo_data = new_repo_data;
```

**Purpose:** When repo N is deleted, all repos with index > N shift down by 1.

**Correctness:** This is correct - HashMap keys (repo indices) are rebuilt.

### PR Row Index Rebuilding - **MISSING**

**Pattern NOT FOUND:** There is NO code that adjusts `selected_prs` when the PR list changes size.

**This should exist but doesn't:**
```rust
// MISSING CODE:
// When PR list shrinks, remove stale indices from selected_prs
let new_selected_prs: Vec<usize> = state.selected_prs
    .iter()
    .filter(|&&idx| idx < data.prs.len())
    .copied()
    .collect();
data.selected_prs = new_selected_prs;
```

### TableState Selection Reclaim Logic (Lines 316-323)

```rust
if data.prs.is_empty() {
    data.table_state.select(None);
    data.selected_prs.clear();
} else if data.table_state.selected().is_none() {
    data.table_state.select(Some(0));
}
```

**Purpose:** When PR list changes, reset or preserve table selection sanely.
**Gap:** Doesn't validate that current selection index is within bounds.

---

## 5. Pain Points Where Indices Become Invalid

### Pain Point 1: Stale selected_prs After Filtering

**Scenario:**
1. Load repo with 20 PRs
2. Select PRs at indices [5, 15, 18]
3. Change filter from "All" to "Feat"
4. New PR list has only 8 PRs (all matching "feat" in title)

**Current Behavior:**
- `selected_prs` still = [5, 15, 18]
- During render, `if selected_prs.contains(&i)` checks never match (i goes 0-7)
- Selected rows visually disappear, but indices remain

**Location:** `src/reducer.rs:310-329` - `Action::RepoDataLoaded`
**Impact:** User sees selected PRs "forgotten" after reload

### Pain Point 2: Stale TableState Selection

**Scenario:**
1. User navigates to PR #9 (row selected)
2. PRs get filtered/reloaded, now only 5 PRs in list
3. TableState still has `Some(9)` selected

**Current Behavior:**
```rust
// No bounds checking in NavigateToNextPr:
let i = match state.state.selected() {
    Some(i) => (i + 1) % state.prs.len(),  // If i=9 and len=5: i=9+1=10, len=5 → 10%5=0 OK
    None => 0,
};
```

**Modulo protects against immediate overflow, but:**
- Initial selection could be out of bounds
- Ratatui's internal rendering might fail silently

**Location:** `src/reducer.rs:368-385` - Navigation wraps properly but initial state isn't validated

### Pain Point 3: Duplicate State Syncing

**Code locations with manual syncing:**
- `src/reducer.rs:270-281` - After repo deletion
- `src/reducer.rs:304-306` - After repo selection
- `src/reducer.rs:415` - After toggling PR selection
- `src/main.rs:651-653` - Load repo state

```rust
// Legacy syncing (Error-prone):
state.prs = data.prs.clone();
state.state = data.table_state.clone();
state.selected_prs = data.selected_prs.clone();
state.loading_state = data.loading_state.clone();
```

**Problem:**
- Two sources of truth for same data
- If one path forgets to sync, state becomes inconsistent
- No compile-time checking that all mutations are synced

**Example of Bug Opportunity:**
```rust
// If merge modifies state.prs but forgets to sync to repo_data:
state.prs.remove(merged_pr_index);
// repo_data.prs is now out of sync!
```

### Pain Point 4: Implicit Index Assumption in Bulk Operations

**Code:** `src/reducer.rs:474-490` - `Action::Rebase`

```rust
let prs_to_rebase: Vec<_> = if state.selected_prs.is_empty() {
    // Auto-rebase: find first PR that needs rebase
    state.prs.iter().filter(...).take(1).cloned().collect()
} else {
    // Rebase selected PRs
    state
        .selected_prs
        .iter()
        .filter_map(|&idx| state.prs.get(idx).cloned())  // ← Bounds checked here!
        .collect()
};
```

**Good:** Uses `.get()` which returns `Option`, safely handling out-of-bounds.
**Bad:** If indices ARE out of bounds, PRs are silently skipped without warning.

**Better approach:** Validate indices before operations or rebuild selected_prs.

### Pain Point 5: Merge Complete Doesn't Rebuild Indices

**Code:** `src/reducer.rs:454-459` - `Action::MergeComplete`

```rust
Action::MergeComplete(Ok(_)) => {
    // Clear selections after successful merge (only if not in merge bot)
    state.selected_prs.clear();
    if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
        data.selected_prs.clear();
    }
}
```

**Flow:**
1. User merges PRs at indices [5, 8, 10]
2. These are removed from PR list
3. All indices > 5 in the list shift down by 3
4. **Existing selected_prs is cleared (OK)**
5. **BUT:** If PR list is NOT reloaded, display shows wrong rows

**Real Bug:** Merge operation doesn't reload PR list automatically!

**Location:** Main merging effect needs to trigger `Action::RefreshCurrentRepo` after completion.

### Pain Point 6: No Validation After Async Loads

**Code:** `src/reducer.rs:310-329` - `Action::RepoDataLoaded` is async

```rust
// Received via channel from background task:
Action::RepoDataLoaded(repo_index, Ok(prs)) => {
    let data = state.repo_data.entry(*repo_index).or_default();
    data.prs = prs.clone();
    
    // selected_prs is inherited from previous state
    // If it contains indices >= prs.len(), they're now invalid
    // ⚠️  NO VALIDATION
}
```

**Validation should happen here:**
```rust
// Validate and repair selected_prs
data.selected_prs.retain(|&idx| idx < data.prs.len());
```

---

## Code Examples of Fragility

### Example 1: Stale Indices Rendered Invisibly

**File:** `src/main.rs:495-530`

```rust
let rows = repo_data.prs.iter().enumerate().map(|(i, item)| {
    let color = if repo_data.selected_prs.contains(&i) {
        app.store.state().theme.selected_bg
    } else {
        color
    };
    ...
});
```

**If `selected_prs = [5, 8, 15]` but `prs.len() = 3`:**
- Loop i goes 0, 1, 2
- Check `selected_prs.contains(&i)` is always false
- Rows never get selected color
- No error, silent corruption

### Example 2: Out-of-Bounds Table Navigation

**File:** `src/reducer.rs:369-379`

```rust
let i = match state.state.selected() {
    Some(i) => {
        if i >= state.prs.len().saturating_sub(1) {
            0
        } else {
            i + 1
        }
    }
    None => 0,
};
state.state.select(Some(i));
```

**Good:** Bounds-checked with modulo-like logic.
**Fragile:** If state.prs is reloaded BETWEEN key press and reducer, desync possible (though unlikely in Rust).

### Example 3: Missing Validation After Reload

**File:** `src/reducer.rs:315-323`

```rust
if data.prs.is_empty() {
    data.table_state.select(None);
    data.selected_prs.clear();
} else if data.table_state.selected().is_none() {
    // Only auto-selects if was empty before
    data.table_state.select(Some(0));
}
// ⚠️ If selected_prs had [8,9,10] and new list is 3 items, indices remain stale
```

### Example 4: Merge Operation Loses Refresh

**File:** `src/effect.rs` (not shown, but described in architecture analysis)

```rust
// After merging PRs, effect likely does:
let _ = app.task_tx.send(BackgroundTask::Merge { ... });

// NO automatic refresh:
// effects.push(Effect::RefreshCurrentRepo);
```

**Result:** PR list displays deleted PRs until user manually refreshes.

---

## Summary of Index Management Complexity

### Current Architecture Problems

1. **Dual storage:** `state.prs` + `repo_data[i].prs` must stay in sync
2. **Dual selection:** `state.selected_prs` + `repo_data[i].selected_prs` must stay in sync
3. **Three indices in play:**
   - Repository index (0..N repos)
   - PR row index (0..M PRs in current repo)
   - PR number (actual GitHub PR #, not positional)

4. **No validation on async updates** - PR lists arrive from background tasks, must validate indices

5. **Filtering changes list size silently** - No index adjustment for selected_prs

6. **Operations don't refresh display** - Merges/rebases don't trigger reload

### Recommended Fixes (Non-Breaking)

1. **Add validation in RepoDataLoaded:**
   ```rust
   data.selected_prs.retain(|&idx| idx < data.prs.len());
   ```

2. **Add auto-refresh after operations:**
   ```rust
   // After merge/rebase completes
   effects.push(Effect::RefreshCurrentRepo);
   ```

3. **Validate on every navigation:**
   ```rust
   if let Some(selected) = state.state.selected() {
       if selected >= state.prs.len() {
           state.state.select(Some(0));
       }
   }
   ```

4. **Consolidate state:** Remove duplicate sync code by using single source of truth

---

## File Reference Summary

- **Index manipulation:** `src/reducer.rs:236-296` (repo deletion), `src/reducer.rs:310-329` (PR reload)
- **Selection toggling:** `src/reducer.rs:404-423`
- **Navigation:** `src/reducer.rs:368-403`
- **Rendering:** `src/main.rs:495-530`
- **Syncing:** Multiple locations, hard to track
- **Gaps/Fragility:** No index validation after async PR load, no auto-refresh after operations

