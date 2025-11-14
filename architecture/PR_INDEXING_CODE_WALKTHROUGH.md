# Detailed Code Walkthrough: Index Management Fragility

## Index Lifecycle & Critical Code Sections

### 1. INITIALIZATION: Loading PRs from GitHub

**Triggering Action:** `Action::BootstrapComplete` or `Action::RefreshCurrentRepo`
**Process:** Background task → GitHub API → `Action::RepoDataLoaded`

**File:** `src/reducer.rs:310-329`
```rust
Action::RepoDataLoaded(repo_index, Ok(prs)) => {
    let data = state.repo_data.entry(*repo_index).or_default();
    data.prs = prs.clone();
    data.loading_state = LoadingState::Loaded;

    // CRITICAL SECTION 1: Selection reclaim logic
    if data.prs.is_empty() {
        // Clear selection when no PRs
        data.table_state.select(None);
        data.selected_prs.clear();
    } else if data.table_state.selected().is_none() {
        // Auto-select first row if nothing selected
        data.table_state.select(Some(0));
    }
    // ⚠️  MISSING: Validate selected_prs indices
    // This is where stale indices from filtered/smaller lists should be cleaned
    
    // Sync legacy fields if this is the selected repo
    if *repo_index == state.selected_repo {
        state.prs = prs.clone();
        state.loading_state = LoadingState::Loaded;
        // ⚠️  MISSING: Also sync state.selected_prs and state.state from repo_data
    }
}
```

**Why This Is Fragile:**
1. `selected_prs` is NOT validated against new `prs.len()`
2. If reloading filtered list, old indices may be out of bounds
3. Sync to `state.prs` exists, but not to `state.selected_prs`
4. Legacy fields can become inconsistent here

**Real Bug Scenario:**
```
Before reload: data.prs=[20 items], data.selected_prs=[5,15,18]
Filter applied: 8 items match filter
After reload: data.prs=[8 items], data.selected_prs=[5,15,18] ← STALE!
```

---

### 2. USER SELECTION: Toggling PR Selection

**Triggering Action:** `Action::TogglePrSelection` (user presses Space)
**File:** `src/reducer.rs:404-423`

```rust
Action::TogglePrSelection => {
    if let Some(selected) = state.state.selected() {
        // Get current selection from TableState
        // ⚠️  selected is Option<usize> from ratatui, not validated yet
        
        if state.selected_prs.contains(&selected) {
            // Remove from selection
            state.selected_prs.retain(|&i| i != selected);
        } else {
            // Add to selection
            state.selected_prs.push(selected);
        }
        // Keep selection sorted for consistent display
        state.selected_prs.sort_unstable();

        // CRITICAL SECTION 2: Sync to repo_data
        if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
            data.selected_prs = state.selected_prs.clone();
            // ⚠️  Cloning vector, not great for perf but correct logic
        }

        // Automatically advance to next PR if not on the last row
        if selected < state.prs.len().saturating_sub(1) {
            effects.push(Effect::DispatchAction(Action::NavigateToNextPr));
        }
    }
}
```

**Why This Is Fragile:**
1. No bounds checking on `selected` before using it
   - If `selected` >= `state.prs.len()`, it still gets added to `selected_prs`
   - This can happen if list shrunk but TableState wasn't updated
2. Two sync points that must succeed:
   - `state.selected_prs` is updated
   - `repo_data[x].selected_prs` must be kept in sync
   - If one fails, state becomes inconsistent

**Bug Path:**
```
1. User is on row 9 (state.selected() = Some(9))
2. Background task reloads with 5 PRs
3. User presses Space before reload completes
4. selected = 9 gets added to selected_prs
5. Rendering loop i=0..4 never matches 9
6. Row appears selected but not highlighted
```

---

### 3. NAVIGATION: Moving Between Rows

**Triggering Actions:** `Action::NavigateToNextPr` or `Action::NavigateToPreviousPr`
**File:** `src/reducer.rs:368-385`

```rust
Action::NavigateToNextPr => {
    let i = match state.state.selected() {
        Some(i) => {
            if i >= state.prs.len().saturating_sub(1) {
                // Wrap around: if at last row, go to first
                0
            } else {
                // Circular navigation: next row
                i + 1
            }
        }
        None => 0,  // If nothing selected, select first
    };
    state.state.select(Some(i));

    // CRITICAL SECTION 3: Sync to repo_data
    if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
        data.table_state.select(Some(i));
        // ⚠️  Only syncs TableState, not selected_prs!
    }
}
```

**Why This Is Fragile:**
1. Bounds checking is done AFTER getting current selection
   - If `state.state.selected()` returns `Some(9)` but list has 5 items, math is:
   - `9 >= 5.saturating_sub(1)` = `9 >= 4` = true → wraps to 0 ✓
   - But only if called in reducer AFTER reload completes
2. No sync of `selected_prs` to `repo_data.selected_prs`
   - Navigating updates one, not the other

**Key Difference from TogglePrSelection:**
- Navigation only touches TableState (focus), not bulk selection
- But still must sync both ways

---

### 4. FILTERING: Applying PR Type Filter

**Triggering Action:** `Action::CycleFilter` (user presses 'f')
**File:** `src/reducer.rs:365-367`

```rust
Action::CycleFilter => {
    state.filter = state.filter.next();
    // ⚠️  THIS IS A DEAD ACTION!
    // Filter change alone doesn't reload PRs
    // No effect is generated!
}
```

**The Real Flow:**
```
1. User presses 'f'
   → Action::CycleFilter updates state.filter (reducer)
   → No effects! Returns empty effects vec

2. Nothing happens to PR list!
   → PRs are still from old filter
   → User has to manually press Ctrl+R to refresh

3. Only when user presses Ctrl+R:
   → Action::RefreshCurrentRepo
   → Effect::LoadSingleRepo spawned
   → GitHub task fetches filtered PRs
   → Action::RepoDataLoaded with new list
```

**Why This Is Fragile:**
1. Filter can change without reloading PRs
2. User must separately trigger refresh
3. If user selected PRs, changes filter, refreshes:
   - `selected_prs` from old list won't match new list indices
   - No automatic cleanup happens

**The Bug That Could Happen:**
```
1. Load 20 PRs: [PR#1..20]
2. Select PRs at indices [5,15,18]
   → selected_prs = [5,15,18]
3. Apply "Feat" filter (doesn't reload yet)
   → selected_prs still = [5,15,18]
   → PRs still = [PR#1..20]
4. Press Ctrl+R to refresh with filter
   → [Background task returns only feature PRs: 8 items]
   → RepoDataLoaded receives [8 new PRs]
   → selected_prs = [5,15,18] ← NOW STALE!
5. Render loop checks selected_prs against new list
   → i=0..7, selected_prs.contains(&i) never matches
   → Selected rows disappear silently
```

---

### 5. REPOSITORY DELETION: Index Rebuild

**Triggering Action:** `Action::DeleteCurrentRepo`
**File:** `src/reducer.rs:236-296`

```rust
Action::DeleteCurrentRepo => {
    if !state.recent_repos.is_empty() {
        let selected_idx = state.selected_repo;

        // Step 1: Remove from list
        state.recent_repos.remove(selected_idx);

        // Step 2: Remove repo data entry
        state.repo_data.remove(&selected_idx);

        // CRITICAL SECTION 4: Rebuild repo indices
        let mut new_repo_data = std::collections::HashMap::new();
        for (old_idx, data) in state.repo_data.iter() {
            let new_idx = if *old_idx > selected_idx {
                old_idx - 1  // Shift down all indices above deleted repo
            } else {
                *old_idx
            };
            new_repo_data.insert(new_idx, data.clone());
        }
        state.repo_data = new_repo_data;

        // Step 3: Adjust selected repo index and sync
        if state.recent_repos.is_empty() {
            state.selected_repo = 0;
            state.prs.clear();
            state.loading_state = LoadingState::Idle;
            state.state.select(None);
            // ⚠️  MISSING: state.selected_prs.clear() here
        } else if selected_idx >= state.recent_repos.len() {
            // Was last repo, select new last one
            state.selected_repo = state.recent_repos.len() - 1;
            if let Some(data) = state.repo_data.get(&state.selected_repo) {
                state.prs = data.prs.clone();
                state.state = data.table_state.clone();
                state.selected_prs = data.selected_prs.clone();  // ✓ Synced
                state.loading_state = data.loading_state.clone();
            }
        } else {
            // Still valid, sync current selection
            if let Some(data) = state.repo_data.get(&state.selected_repo) {
                state.prs = data.prs.clone();
                state.state = data.table_state.clone();
                state.selected_prs = data.selected_prs.clone();  // ✓ Synced
                state.loading_state = data.loading_state.clone();
            }
        }
    }
}
```

**This Part Is Actually Correct!**
- Repository indices (HashMap keys) are properly rebuilt
- When repo N is deleted, repos with index > N become index-1
- State is re-synced after deletion
- `selected_prs` contains PR row indices (not repo indices), so no adjustment needed

**But There's Still A Fragility:**
- Repository deletion modifies `repo_data` HashMap
- If another thread/task is iterating `repo_data`, this causes UB in non-concurrent code
- The pattern of removing entry, rebuilding all others is inefficient

---

### 6. MERGE COMPLETION: No Auto-Refresh

**Triggering Action:** `Action::MergeComplete(Ok(...))`
**File:** `src/reducer.rs:454-459`

```rust
Action::MergeComplete(Ok(_)) => {
    // Clear selections after successful merge (only if not in merge bot)
    state.selected_prs.clear();
    if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
        data.selected_prs.clear();
        // ⚠️  MISSING: state.prs is NOT updated!
    }
    // ⚠️  MISSING: No effect generated to refresh PR list!
}
```

**Why This Is Fragile:**
1. Merged PRs are deleted from GitHub
2. But local `state.prs` still contains them
3. No effect is generated to reload (e.g., `Effect::RefreshCurrentRepo`)
4. Display shows stale/deleted PRs until user manually refreshes

**The Bug Scenario:**
```
1. User selects PRs #5, #8, #10 (all active)
   → selected_prs = [0, 2, 4] (row indices)
   → state.prs = [PR#5, PR#8, PR#10, ...]

2. User presses 'm' to merge
   → Action::MergeSelectedPrs
   → Effect::PerformMerge spawned
   → Background task merges PRs at indices [0,2,4]

3. Merge completes successfully
   → Action::MergeComplete(Ok(...))
   → Reducer clears selected_prs ✓
   → BUT state.prs still has old PR objects! ⚠️

4. Rendering shows all original PRs
   → PR#5, PR#8, PR#10 appear "merged but still in list"
   → User must Ctrl+R to see actual state
```

**The Fix:**
```rust
Action::MergeComplete(Ok(_)) => {
    state.selected_prs.clear();
    if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
        data.selected_prs.clear();
    }
    // ADD THIS:
    effects.push(Effect::RefreshCurrentRepo);
}
```

---

### 7. RENDERING: Silent Corruption

**File:** `src/main.rs:495-530`

```rust
let rows = repo_data.prs.iter().enumerate().map(|(i, item)| {
    let color = match i % 2 {
        0 => app.store.state().repos.colors.normal_row_color,
        _ => app.store.state().repos.colors.alt_row_color,
    };
    
    // CRITICAL SECTION 5: Selection color application
    let color = if repo_data.selected_prs.contains(&i) {
        app.store.state().theme.selected_bg
    } else {
        color
    };
    
    let row: Row = item.into();
    row.style(
        Style::new()
            .fg(app.store.state().repos.colors.row_fg)
            .bg(color),
    )
    .height(1)
});
```

**The Problem With This Code:**
- Assumes every index i ∈ [0..prs.len()) is correct
- If `selected_prs` contains indices >= prs.len(), they're silently ignored
- No warning, no error, just invisible selection

**Example of Silent Corruption:**
```
selected_prs = [5, 8, 15]
prs.len() = 3

Render loop:
  i=0: contains(&0)? No → normal color
  i=1: contains(&1)? No → normal color
  i=2: contains(&2)? No → normal color
  // Loop ends, never checks i=5,8,15
  
Result: User sees nothing selected, but state says 3 rows are selected
```

**Why This Is Especially Bad:**
1. No assertion or panic → appears to work
2. No warning to developer → hard to debug
3. Display corruption is silent → user confused
4. Might happen intermittently after filtering

---

### 8. SYNC POINTS: Manual Duplication

**Problem Pattern Found In Multiple Locations:**

**Location 1:** After repo selection `src/reducer.rs:297-309`
```rust
Action::SelectRepoByIndex(index) => {
    if *index < state.recent_repos.len() {
        state.selected_repo = *index;
        
        // Manual sync from repo_data to state
        if let Some(data) = state.repo_data.get(index) {
            state.prs = data.prs.clone();
            state.state = data.table_state.clone();
            state.selected_prs = data.selected_prs.clone();
            state.loading_state = data.loading_state.clone();
        }
    }
}
```

**Location 2:** After repo deletion `src/reducer.rs:269-283`
```rust
if let Some(data) = state.repo_data.get(&state.selected_repo) {
    state.prs = data.prs.clone();
    state.state = data.table_state.clone();
    state.selected_prs = data.selected_prs.clone();
    state.loading_state = data.loading_state.clone();
}
```

**Location 3:** After PR toggle `src/reducer.rs:413-416`
```rust
if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
    data.selected_prs = state.selected_prs.clone();
}
```

**Location 4:** In main.rs `src/main.rs:649-671`
```rust
if let Some(data) = state.repo_data.get(&state.selected_repo) {
    state.prs = data.prs.clone();
    state.state = data.table_state.clone();
    state.selected_prs = data.selected_prs.clone();
    state.loading_state = data.loading_state.clone();
}
```

**Why This Is Fragile:**
1. Same pattern repeated 4+ times
2. If one location forgets a field, inconsistency spreads
3. Example: Location 1 syncs prs/state/selected_prs/loading_state
   - If Location 3 only syncs selected_prs, other fields diverge
4. No compile-time checking that all fields are synced

**Solution: Extract Helper**
```rust
fn sync_from_repo_data(state: &mut ReposState, repo_index: usize) {
    if let Some(data) = state.repo_data.get(&repo_index) {
        state.prs = data.prs.clone();
        state.state = data.table_state.clone();
        state.selected_prs = data.selected_prs.clone();
        state.loading_state = data.loading_state.clone();
    }
}

// Then in reducer:
Action::SelectRepoByIndex(index) => {
    if *index < state.recent_repos.len() {
        state.selected_repo = *index;
        sync_from_repo_data(&mut state, *index);  // Single call
    }
}
```

---

## Summary Table: Fragility Locations

| Section | File:Lines | Issue Type | Severity |
|---------|-----------|-----------|----------|
| Initialization | reducer.rs:310-329 | No index validation after reload | HIGH |
| Selection Toggle | reducer.rs:404-423 | No bounds check on selected | HIGH |
| Navigation | reducer.rs:368-385 | Missing selected_prs sync | MEDIUM |
| Filtering | reducer.rs:365-367 | Dead action, no auto-reload | MEDIUM |
| Repo Delete | reducer.rs:247-257 | Inefficient rebuild pattern | LOW |
| Merge Complete | reducer.rs:454-459 | No auto-refresh | HIGH |
| Rendering | main.rs:495-530 | Silent index corruption | HIGH |
| Sync Points | Multiple | Code duplication | MEDIUM |

---

## The Bigger Picture

### Why Dual State Exists
1. Original design: `ReposState` was the main store
2. Refactoring added: Multi-repo support via `RepoData` HashMap
3. Migration incomplete: Both systems coexist, must be manually synced

### Why Sync Is Manual
1. No unified reducer pattern for the HashMap
2. `state.prs` is legacy, `repo_data[i].prs` is new
3. Both must stay in sync, but sync is ad-hoc
4. No abstraction layer to enforce consistency

### Why Index Validation Is Missing
1. Framework assumption: Once list is loaded, it doesn't change size
2. Filtering wasn't part of original design
3. Index validation added incrementally but incompletely
4. Testing may not cover filter-then-reload-then-select scenario

