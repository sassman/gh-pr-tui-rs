# Architecture Analysis: Data Flow Issues

## Current Problems

### 1. Mixed Data Flow Sources
Currently, actions can be dispatched from 3 different places:
- **User input** → `handle_key_event()` → returns `Action`
- **Effect execution** → `execute_effect()` → calls `action_tx.send()` (19 times)
- **Background tasks** → worker tasks → call `action_tx.send()` (30 times)

This creates a complex, hard-to-trace data flow:
```
User Input ──→ Action ──→ Reducer ──→ Effects ──┐
                  ↑                              │
                  │                              ↓
                  └──────── execute_effect() ────┤
                  ↑                              │
                  │                              ↓
                  └──────── background_tasks ────┘
```

### 2. Violations of Redux/Elm Pattern

**Reducer (✓ GOOD):**
- Pure functions
- Return `(State, Vec<Effect>)`
- No side effects, no action dispatching

**Effects (✗ BAD):**
- Execute side effects (good)
- BUT ALSO dispatch actions directly (bad)
- Mixes concerns: execution + event generation

**Background Tasks (✗ BAD):**
- Long-running operations (good)
- BUT dispatch actions directly (bad)
- Bypasses the reducer completely

### 3. Specific Issues in Current Code

#### In `effect.rs`:
```rust
// Example: SetTaskStatus actions dispatched directly
let _ = app.action_tx.send(Action::SetTaskStatus(Some(TaskStatus {
    message: format!("..."),
    status_type: TaskStatusType::Running,
})));

// Example: AddToAutoMergeQueue dispatched directly
let _ = app.action_tx.send(Action::AddToAutoMergeQueue(repo_index, pr_number));
```

**Problem**: Effects should describe what to do, not dispatch follow-up actions. This creates implicit action chains that are hard to trace.

#### In `task.rs`:
```rust
// Example: Background task dispatching actions directly
let _ = action_tx.send(Action::RepoDataLoaded(repo_index, Ok(prs)));
let _ = action_tx.send(Action::MergeStatusUpdated(repo_index, pr_number, status));
```

**Problem**: Background tasks have direct access to dispatch any action, bypassing reducer logic.

#### In `task.rs` auto-merge monitoring:
```rust
tokio::spawn(async move {
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let _ = action_tx_clone.send(Action::AutoMergeStatusCheck(...));
        // More action dispatching...
    }
});
```

**Problem**: Long-running spawned task dispatches actions over time, creating complex timing dependencies.

---

## Proposed Clean Architecture

### Pure Unidirectional Data Flow:

```
User Input ──→ Action ──→ Reducer ──→ (State, Effects)
                  ↑                         │
                  │                         ↓
                  │                   Execute Effects
                  │                         │
                  │                         ↓
                  │              ┌─────────────────┐
                  │              │ Task Results    │
                  │              │ Channel         │
                  │              └─────────────────┘
                  │                         │
                  └─────────────────────────┘
                    (Convert Result → Action)
```

### Key Principles:

1. **Reducers**: Pure functions, no side effects
2. **Effects**: Describe side effects, return completion results
3. **Background Tasks**: Send results to a channel, don't dispatch actions
4. **Main Loop**: Only place that dispatches actions (from user input or task results)

---

## Recommended Refactoring

### Phase 1: Separate Task Results from Actions

Create a new `TaskResult` enum for background task completions:

```rust
// src/task.rs
pub enum TaskResult {
    BootstrapComplete(Result<BootstrapResult, String>),
    RepoDataLoaded(usize, Result<Vec<Pr>, String>),
    MergeComplete(Result<(), String>),
    AutoMergeStatusUpdate(usize, usize, MergeableStatus), // repo_index, pr_number, status
    // ... etc
}
```

Change background tasks to send `TaskResult` instead of `Action`:

```rust
// OLD:
let _ = action_tx.send(Action::RepoDataLoaded(repo_index, Ok(prs)));

// NEW:
let _ = result_tx.send(TaskResult::RepoDataLoaded(repo_index, Ok(prs)));
```

### Phase 2: Main Loop Converts Results to Actions

```rust
// src/main.rs - main loop
loop {
    tokio::select! {
        // Handle user actions
        Some(action) = action_rx.recv() => {
            let effects = app.store.dispatch(action);
            for effect in effects {
                execute_effect(app, effect).await?;
            }
        }

        // Handle task results - convert to actions
        Some(result) = task_result_rx.recv() => {
            let action = result_to_action(result);
            let effects = app.store.dispatch(action);
            for effect in effects {
                execute_effect(app, effect).await?;
            }
        }
    }
}

fn result_to_action(result: TaskResult) -> Action {
    match result {
        TaskResult::RepoDataLoaded(idx, data) => Action::RepoDataLoaded(idx, data),
        TaskResult::MergeComplete(res) => Action::MergeComplete(res),
        // ... simple 1:1 mapping
    }
}
```

### Phase 3: Effects Return Action Chains

Instead of dispatching actions, effects return additional actions to dispatch:

```rust
// src/effect.rs
pub async fn execute_effect(app: &mut App, effect: Effect) -> Result<Vec<Action>> {
    let mut follow_up_actions = Vec::new();

    match effect {
        Effect::EnableAutoMerge { repo_index, repo, pr_number } => {
            // Add to queue - return action instead of dispatching
            follow_up_actions.push(Action::AddToAutoMergeQueue(repo_index, pr_number));

            // Set status - return action
            follow_up_actions.push(Action::SetTaskStatus(Some(TaskStatus {
                message: format!("Enabling auto-merge for PR #{}...", pr_number),
                status_type: TaskStatusType::Running,
            })));

            // Spawn background task (still uses task_tx, but returns results)
            let _ = app.task_tx.send(BackgroundTask::EnableAutoMerge {
                repo_index, repo, pr_number,
                octocrab: app.octocrab()?,
            });
        }
        // ...
    }

    Ok(follow_up_actions)
}
```

Then main loop dispatches these:

```rust
for effect in effects {
    let follow_up_actions = execute_effect(app, effect).await?;
    for action in follow_up_actions {
        let new_effects = app.store.dispatch(action);
        // Continue until no more effects...
    }
}
```

### Phase 4: Long-Running Monitoring Tasks

For tasks like auto-merge monitoring that need to send periodic updates:

```rust
// Instead of spawning a task that dispatches actions:
tokio::spawn(async move {
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let _ = action_tx.send(Action::AutoMergeStatusCheck(...)); // BAD
    }
});

// Use a monitoring task that sends results:
tokio::spawn(async move {
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_secs(60)).await;

        // Check status and send result
        if let Ok(status) = check_pr_status(&octocrab, &repo, pr_number).await {
            let _ = result_tx.send(TaskResult::AutoMergeStatusUpdate(
                repo_index, pr_number, status
            ));
        }
    }
});
```

---

## Benefits of Clean Architecture

1. **Traceable Data Flow**: Actions only come from 2 sources (user input, task results)
2. **Predictable State Changes**: All state changes go through reducers
3. **Testable**: Reducers are pure functions, easy to test
4. **Debuggable**: Can log all actions/effects in one place
5. **Time-Travel Debugging**: Possible to replay actions since state changes are deterministic

---

## Migration Strategy

### Step 1: Add TaskResult channel (low risk)
- Add `task_result_rx` channel to main loop
- Both Action and TaskResult channels exist temporarily

### Step 2: Convert task.rs to use results (medium risk)
- Change background tasks one-by-one to send TaskResult
- Add result_to_action() converter
- Test each task type

### Step 3: Remove action dispatching from effects (medium risk)
- Change execute_effect to return Vec<Action>
- Update main loop to dispatch returned actions
- Test effect chains

### Step 4: Clean up (low risk)
- Remove action_tx from background tasks
- Remove action_tx.send() from effect.rs
- Update documentation

---

## Current Auto-Merge Flow Analysis

The auto-merge feature is a perfect example of the complexity:

```
User presses 'm'
  → Action::MergeSelectedPrs
  → Reducer checks if PR is building
  → Returns Effect::EnableAutoMerge
  → execute_effect dispatches Action::AddToAutoMergeQueue  (❌ should return this)
  → execute_effect dispatches Action::SetTaskStatus  (❌ should return this)
  → execute_effect spawns BackgroundTask::EnableAutoMerge
  → Background task calls GitHub API
  → Background task dispatches Action::SetTaskStatus  (❌ should send result)
  → Background task spawns monitoring task
  → Monitoring task runs for 20 minutes
  → Every minute, dispatches Action::AutoMergeStatusCheck  (❌ should send result)
  → Every minute, dispatches Action::MergeStatusUpdated  (❌ should send result)
  → Eventually dispatches Action::RemoveFromAutoMergeQueue  (❌ should send result)
```

**With clean architecture:**

```
User presses 'm'
  → Action::MergeSelectedPrs
  → Reducer checks if PR is building
  → Returns Effect::EnableAutoMerge
  → execute_effect returns [Action::AddToAutoMergeQueue, Action::SetTaskStatus]
  → Main loop dispatches these actions
  → execute_effect spawns BackgroundTask::EnableAutoMerge
  → Background task calls GitHub API
  → Background task sends TaskResult::AutoMergeEnabled(Result)
  → Main loop converts to Action::SetTaskStatus
  → Background task starts monitoring
  → Every minute, sends TaskResult::AutoMergeStatusUpdate
  → Main loop converts to Action::AutoMergeStatusCheck
  → Reducer checks status and returns Effect::PerformMerge when ready
```

Much cleaner and traceable!

---

## Recommendation

**Do the refactoring in phases** to avoid breaking everything at once. The architecture debt is real and makes debugging harder, but it's fixable with incremental changes.

Start with Phase 1 (TaskResult channel) as it's the foundation for the cleaner architecture.
