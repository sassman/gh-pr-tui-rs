# Architecture Documentation

This folder contains architectural analysis and design documents for the pr-bulk-review-tui project.

## Documents Overview

### 1. Clean Architecture Pattern
**ARCHITECTURE_ANALYSIS.md** - Redux/Elm architecture implementation
- Unidirectional data flow: Action → Reducer → Effect → TaskResult → Action
- Separation of concerns: pure reducers, side effects in effects
- Background task system with result channels
- Clean separation between business logic and I/O

### 2. PR Selection & Indexing Analysis
Focus: Current implementation analysis and identified issues

**INDEX_ANALYSIS_README.md** - Navigation guide
- Quick overview of all indexing documents
- Where to start based on your needs

**PR_INDEXING_SUMMARY.md** ⭐ **START HERE**
- Quick reference for the main issues
- 6 critical pain points with line numbers
- Prioritized fix recommendations
- Visual diagrams of state duplication

**PR_INDEXING_ANALYSIS.md** - Detailed technical breakdown
- How selected_prs and TableState work
- Where indices are used (13+ locations)
- Specific bugs with code examples
- State transition scenarios

**PR_INDEXING_CODE_WALKTHROUGH.md** - Line-by-line analysis
- 8 critical code sections explained
- Rendering logic (main.rs:495-530)
- Selection toggle (reducer.rs:404-423)
- Data loading validation gaps (reducer.rs:310-329)

### 3. PR Selection & Indexing Proposals
**PR_INDEXING_PROPOSALS.md** ⭐ **SOLUTIONS DOCUMENT**
- **Proposal 1: PR Number-Based Selection** (Recommended)
  - Use GitHub PR numbers instead of array indices
  - Performance: O(n) vs current O(n*k)
  - Fixes all stale index bugs automatically
  - Easy migration path with parallel tracking

- **Proposal 2: Stable ID System** (Future consideration)
  - Unique IDs per PR: "owner/repo#number"
  - Most robust, supports cross-repo features
  - Higher complexity, recommended for v2.0

- **Proposal 3: Cursor-Only Selection** (Minimal)
  - Remove multi-select, cursor position only
  - Simplest but removes functionality

- **Proposal 4: Hybrid Approach** (Advanced)
  - PR number-based selection + stable cursor
  - Best UX but more complexity

## Reading Guide

### If you want to understand the current issues:
1. Read **PR_INDEXING_SUMMARY.md** (10 min)
2. Look at specific pain points in **PR_INDEXING_ANALYSIS.md** (20 min)
3. See concrete code examples in **PR_INDEXING_CODE_WALKTHROUGH.md** (30 min)

### If you want to implement fixes:
1. Read **PR_INDEXING_PROPOSALS.md** → "Proposal 1: PR Number-Based Selection"
2. Follow the implementation roadmap (4 sprints)
3. Review "Code Locations to Change" table
4. See "Proof of Concept" for before/after code

### If you're new to the project:
1. Start with **ARCHITECTURE_ANALYSIS.md** to understand Redux pattern
2. Read **PR_INDEXING_SUMMARY.md** to see current pain points
3. Review **PR_INDEXING_PROPOSALS.md** for the recommended solution

## Key Findings Summary

### Root Cause
**Dual parallel state** with manual synchronization:
- `ReposState` has: prs, state, selected_prs (legacy)
- `RepoData` has: prs, table_state, selected_prs (modern)
- Must manually sync at 4+ locations
- Array indices become invalid when PR list changes

### 6 Critical Bugs Identified

| Priority | Issue | Location | Impact |
|----------|-------|----------|--------|
| HIGH | Stale indices after filtering | reducer.rs:310-329 | Silent data corruption |
| HIGH | No TableState validation | reducer.rs:316-323 | Invalid cursor position |
| HIGH | Silent index corruption | main.rs:495-530 | Selection disappears |
| MEDIUM | No auto-refresh after merge | reducer.rs:454-459 | Stale UI data |
| MEDIUM | Duplicate sync code | 4+ locations | Error-prone maintenance |
| MEDIUM | Filter doesn't reload | reducer.rs:365-367 | Manual refresh needed |

### Recommended Solution

**Migrate to PR number-based selection:**
- Replace `selected_prs: Vec<usize>` (indices)
- With `selected_pr_numbers: HashSet<usize>` (GitHub PR numbers)

**Benefits:**
- ✅ Fixes all 6 bugs automatically
- ✅ Better performance (O(n) vs O(n*k))
- ✅ No index validation needed
- ✅ Works with filtering/sorting naturally
- ✅ Easy migration (run both systems in parallel)

**Migration Roadmap:** See PR_INDEXING_PROPOSALS.md → "Implementation Roadmap"

## File Change Impact

### Files with Index-Related Code
- `src/state.rs` - State definitions (add selected_pr_numbers)
- `src/reducer.rs` - 8 actions dealing with selection
- `src/main.rs` - Rendering and event handling
- `src/shortcuts.rs` - Selection toggle logic

### Expected Changes
- **Lines added:** ~50 (new HashSet tracking)
- **Lines removed:** ~30 (sync code elimination)
- **Complexity reduction:** 20% (no validation needed)
- **Performance improvement:** 2-5x (rendering loop)

## Timeline Estimates

### Quick Fixes (High Priority Bugs)
**1-2 days:**
- Add index validation in RepoDataLoaded
- Auto-refresh after merge operations
- Bounds check before operations

### Full Migration (PR Number-Based)
**1-2 weeks:**
- Sprint 1: Add parallel tracking (2 days)
- Sprint 2: Switch rendering (2 days)
- Sprint 3: Update operations (2 days)
- Sprint 4: Remove old system (1 day)

### State Consolidation (Remove Duplication)
**2-3 days:**
- Remove ReposState legacy fields
- Update all access patterns to use repo_data
- Test thoroughly

## Testing Checklist

After implementing PR number-based selection, verify:

- [ ] Select PRs, apply filter → selection preserved correctly
- [ ] Select PRs, reload → selection restored if PR still exists
- [ ] Select PRs, merge → selection cleared (or kept if PR remains)
- [ ] Navigate with arrows → cursor stable across filter changes
- [ ] Multi-select 10 PRs → rendering performance acceptable
- [ ] Switch repos → selection isolated per repo
- [ ] Session persistence → selections survive app restart

## Related Issues

See PR_INDEXING_SUMMARY.md "Code Fragility Examples" for:
- Invisible corruption scenario (filter → reload)
- Forgotten sync bug pattern
- Filter change dead code

## Questions?

- **Why not just fix the validation?** Band-aid solution, doesn't address root cause
- **Why PR numbers over stable IDs?** Simpler, addresses current bugs, can evolve later
- **What about performance?** HashSet lookup is O(1), actually faster than current system
- **Breaking changes?** Session persistence format changes, but migration is automatic

---

*Last updated: 2025-11-14*
*Status: Analysis complete, proposals ready for implementation*
