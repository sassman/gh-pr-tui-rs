# PR Selection and Indexing Analysis - Complete Report

## Document Overview

This directory contains a comprehensive analysis of how PR selection and indexing works in the sassman codebase, focusing on fragility points and potential bugs.

### Files Included

1. **PR_INDEXING_SUMMARY.md** (8.2 KB) - Start here!
   - Quick reference guide
   - Six critical pain points at a glance
   - Recommended fixes prioritized by severity
   - Best for getting oriented quickly

2. **PR_INDEXING_ANALYSIS.md** (16 KB) - Detailed reference
   - Complete technical breakdown
   - All code locations referenced
   - Specific code examples showing fragility
   - Why indices become invalid
   - File reference summary

3. **PR_INDEXING_CODE_WALKTHROUGH.md** (16 KB) - Deep dive
   - Line-by-line code analysis
   - Eight critical sections explained
   - Real bug scenarios with step-by-step walkthroughs
   - Why dual state exists and why it's fragile
   - Sync pattern analysis with solutions

---

## Key Findings Summary

### The Problem: Dual Parallel State

The codebase manages PR selection with two independent data structures that must stay in sync:

```
ReposState (Legacy)          RepoData (Modern)
├─ prs: Vec<Pr>      ←sync→  ├─ prs: Vec<Pr>
├─ state: TableState  ←sync→  ├─ table_state: TableState
└─ selected_prs      ←sync→  └─ selected_prs
```

**Problem:** Manual syncing between these creates 4+ error-prone sync points.

### Six Critical Pain Points

1. **Stale selected_prs After Filtering** (HIGH)
   - When PR list is filtered and reloaded, indices become invalid
   - Location: `src/reducer.rs:310-329`
   - Fix: Add `data.selected_prs.retain(|&idx| idx < data.prs.len());`

2. **No TableState Validation on Reload** (HIGH)
   - Selected row not bounds-checked after PR list changes
   - Location: `src/reducer.rs:316-323`
   - Fix: Reset or validate selection when list size changes

3. **Silent Index Failures at Rendering** (HIGH)
   - Stale indices silently ignored with no warning
   - Location: `src/main.rs:495-530`
   - Result: User sees selected PRs "disappear"

4. **No Auto-Refresh After Operations** (HIGH)
   - Merge/rebase don't reload PR list automatically
   - Location: `src/reducer.rs:454-459`
   - Result: Display shows merged PRs until user Ctrl+R

5. **Duplicate Sync Code** (MEDIUM)
   - Same pattern repeated 4+ times without central validation
   - Locations: `src/reducer.rs` multiple places, `src/main.rs:651-653`
   - Risk: Forgetting one field causes inconsistency

6. **Filtering Doesn't Auto-Reload** (MEDIUM)
   - Filter changes alone don't trigger PR reload
   - Location: `src/reducer.rs:365-367`
   - User must separately press Ctrl+R

---

## Three Levels of Index Complexity

### Level 1: Repository Index (Stable)
- Maps to repo in `recent_repos` vector
- Properly rebuilt when repo deleted (lines 247-257)
- Uses HashMap with index as key

### Level 2: PR Row Index (Fragile)
- Position in current repo's PR list (0..M)
- Stored in `selected_prs` vector
- **NO validation** when list size changes
- **NO automatic adjustment** when filtering/reloading

### Level 3: PR Number (Stable)
- Actual GitHub PR# (e.g., #123)
- Used correctly in API calls
- Operations should use this instead of row index

---

## Real Bug Scenario

```
1. Load 20 PRs from GitHub
2. User selects rows [5, 15, 18]
   → selected_prs = [5, 15, 18]

3. User applies "Feat" filter and refreshes (Ctrl+R)
   → Background task fetches filtered PRs: 8 items
   → RepoDataLoaded action received
   → selected_prs still = [5, 15, 18] ← STALE!

4. Rendering loop checks containment
   → for i in 0..8: contains(&i)? Never matches 5, 15, 18
   → Selected rows appear unselected but state thinks they're selected

5. If user presses Space thinking row 0 is selected
   → Actually toggles something different
   → State corruption spreads
```

---

## Architecture Lessons

### Why Dual State Exists
1. Original design: Single repo with `ReposState`
2. Later: Multi-repo support added via `RepoData` HashMap
3. Migration incomplete: Both coexist with manual sync

### Why It's Fragile
1. **No type system enforcement** - Sync is manual, not automatic
2. **Pattern repetition** - Same 4-line sync code duplicated 4+ times
3. **No validation** - Index bounds not checked after async updates
4. **Silent failures** - Stale indices silently ignored at render time
5. **Async boundaries** - PR lists arrive from background tasks with no validation

### Why It Still Mostly Works
1. Simple operations (navigate, toggle) mostly sync correctly
2. Most users don't trigger filter→reload→select sequence
3. Crashes would be easier to notice than silent corruption
4. Modulo arithmetic and saturation math provide some protection

---

## Recommended Fixes (Priority Order)

### HIGH PRIORITY (Prevents Silent Data Corruption)
```rust
// Fix 1: Validate indices after PR list reload (Line 329)
data.selected_prs.retain(|&idx| idx < data.prs.len());
data.table_state.select(None);  // Reset focus safely

// Fix 2: Auto-refresh after merge (Line 554)
effects.push(Effect::RefreshCurrentRepo);

// Fix 3: Auto-refresh after rebase (similar location)
effects.push(Effect::RefreshCurrentRepo);
```

### MEDIUM PRIORITY (Prevent Future Bugs)
```rust
// Fix 4: Extract sync helper (currently in 4+ places)
fn sync_repo_state(state: &mut ReposState, repo_idx: usize) {
    if let Some(data) = state.repo_data.get(&repo_idx) {
        state.prs = data.prs.clone();
        state.state = data.table_state.clone();
        state.selected_prs = data.selected_prs.clone();
        state.loading_state = data.loading_state.clone();
    }
}

// Fix 5: Add bounds validation before rebase
selected_prs.retain(|&idx| idx < prs.len());
```

### LOW PRIORITY (Code Quality)
```rust
// Fix 6: Long-term - consolidate to single source of truth
// Remove state.prs, state.state, state.selected_prs
// Use only repo_data[selected_repo].* with getter functions
```

---

## Files to Watch During Refactoring

| Component | File | Lines | Risk Level |
|-----------|------|-------|-----------|
| PR reload validation | reducer.rs | 310-329 | HIGH |
| TableState bounds | reducer.rs | 316-323 | HIGH |
| Merge completion | reducer.rs | 454-459 | HIGH |
| Rendering | main.rs | 495-530 | HIGH |
| Sync duplication | reducer.rs, main.rs | Multiple | MEDIUM |
| Selection toggle | reducer.rs | 404-423 | MEDIUM |
| Navigation sync | reducer.rs | 368-403 | MEDIUM |
| Filter change | reducer.rs | 365-367 | MEDIUM |

---

## Quick Check: Is Your Code Affected?

Are you experiencing:
- Selected PRs disappearing after filtering?
- Merged PRs still showing in list?
- Selected row count not matching highlighted rows?
- Inconsistent state between different screens?

→ These are likely manifestations of the issues documented here.

---

## How These Documents Relate

```
START HERE
    ↓
PR_INDEXING_SUMMARY.md (overview + fixes)
    ↓ (need details?)
PR_INDEXING_ANALYSIS.md (technical reference)
    ↓ (need code examples?)
PR_INDEXING_CODE_WALKTHROUGH.md (line-by-line analysis)
    ↓
(Come back to this README for navigation)
```

---

## Next Steps

1. **Read PR_INDEXING_SUMMARY.md** - Get oriented (5 min)
2. **Identify your symptom** in the pain points section
3. **Read relevant section** in PR_INDEXING_CODE_WALKTHROUGH.md (10 min)
4. **Review the fix** in PR_INDEXING_SUMMARY.md recommended fixes section
5. **Use PR_INDEXING_ANALYSIS.md** as reference while coding the fix

---

## Questions?

The analysis includes:
- Line-by-line code walkthrough of each fragile section
- Real bug scenarios with step-by-step execution flows
- Specific code snippets showing the problem and solution
- Summary table of all fragility locations and their severity

Refer to the appropriate document for the level of detail needed.

---

## Document Generation

Created: 2025-11-14
Analysis Depth: Medium (comprehensive but focused on fragility points)
Total Content: ~1,300 lines across 3 documents
Code Examples: 30+ specific snippets with line numbers

