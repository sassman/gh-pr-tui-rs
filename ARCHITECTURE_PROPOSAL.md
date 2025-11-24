# Redux Architecture Refactoring Proposal

## Current Problems Summary

Based on comprehensive analysis, the current architecture has these critical issues:

1. **Impure Reducers**: Return `(State, Vec<Effect>)` instead of just `State`
2. **Effect Chaining**: `Effect::DispatchAction` creates unbounded action loops
3. **Three Separate Concepts**: Actions, Effects, BackgroundTasks
4. **Manual Mapping**: TaskResult → Action conversion (21 variants)
5. **3-Level Recursion**: Action → Effect → Action → Effect → Action
6. **Massive Reducer**: 2,211 lines, 1,250 in one sub-reducer alone
7. **Hidden Control Flow**: Effects dispatching actions makes flow hard to trace

**Metrics**:
- 152 Actions
- 40+ Effects
- 11 BackgroundTask types
- 21 TaskResult types
- 7 sub-reducers with different signatures
- 3+ levels of dispatch recursion

---

## Proposed Architecture: Middleware-Based Redux

### Core Principles

1. **Pure Reducers**: `(State, &Action) → State` - no side effects
2. **Middleware Chain**: Handles side effects, async operations, logging
3. **Unified Actions**: No separate BackgroundTask/TaskResult concepts
4. **Single Source of Truth**: All state changes go through one clear path
5. **Traceable Flow**: Action → Middleware → Reducer → State

### Architecture Diagram

```
User Event
    ↓
 Action
    ↓
┌─────────────────────┐
│  Middleware Chain   │
├─────────────────────┤
│  1. Logging         │ ← Logs all actions
│  2. Thunk           │ ← Handles async actions
│  3. Task Manager    │ ← Manages background tasks
│  4. Cache           │ ← Caching layer
│  5. Analytics       │ ← Metrics/telemetry
└─────────────────────┘
    ↓
 Reducer (Pure)
    ↓
  State
    ↓
  View
```

---

## Detailed Design

### 1. Pure Reducer Signature

**Before** (impure):
```rust
pub fn reduce(state: AppState, action: &Action) -> (AppState, Vec<Effect>) {
    let mut effects = Vec::new();
    // ... business logic ...
    effects.push(Effect::LoadRepos { ... });
    (state, effects)
}
```

**After** (pure):
```rust
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    // Only transforms state, no side effects
    match action {
        Action::RepoDataLoaded(idx, Ok(prs)) => {
            if let Some(data) = state.repos.repo_data.get_mut(idx) {
                data.prs = prs.clone();
                data.loading_state = LoadingState::Loaded;
                data.last_updated = Some(chrono::Local::now());
            }
        }
        // ... more patterns ...
    }
    state
}
```

### 2. Middleware Trait

```rust
/// Middleware can intercept actions before they reach the reducer
/// and dispatch new actions asynchronously
pub trait Middleware: Send + Sync {
    /// Called for every action before it reaches the reducer
    ///
    /// Can:
    /// - Inspect the action
    /// - Dispatch new actions via dispatcher
    /// - Perform side effects
    /// - Block the action (return false)
    ///
    /// Returns: true to continue, false to block action
    fn handle(
        &mut self,
        action: &Action,
        state: &AppState,
        dispatcher: &Dispatcher,
    ) -> BoxFuture<'_, bool>;
}

/// Dispatcher allows middleware to dispatch new actions
pub struct Dispatcher {
    tx: mpsc::UnboundedSender<Action>,
}

impl Dispatcher {
    pub fn dispatch(&self, action: Action) {
        let _ = self.tx.send(action);
    }

    pub fn dispatch_async(&self, action: Action) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(action);
        });
    }
}
```

### 3. Store with Middleware

```rust
pub struct Store {
    state: AppState,
    middleware: Vec<Box<dyn Middleware>>,
    dispatcher: Dispatcher,
}

impl Store {
    pub fn new(initial_state: AppState) -> (Self, mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher { tx };

        Self {
            state: initial_state,
            middleware: Vec::new(),
            dispatcher,
        }
    }

    pub fn add_middleware<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middleware.push(Box::new(middleware));
    }

    /// Dispatch action through middleware chain, then reducer
    pub async fn dispatch(&mut self, action: Action) {
        // Run through middleware chain
        let mut should_continue = true;
        for middleware in &mut self.middleware {
            if !middleware.handle(&action, &self.state, &self.dispatcher).await {
                should_continue = false;
                break;
            }
        }

        // If not blocked, apply to reducer
        if should_continue {
            self.state = reduce(self.state.clone(), &action);
        }
    }
}
```

### 4. Example Middleware: Task Manager

Replaces the entire `task.rs` + `effect.rs` system:

```rust
pub struct TaskMiddleware {
    octocrab: Option<Octocrab>,
    cache: Arc<Mutex<GitHubApiCache>>,
}

impl Middleware for TaskMiddleware {
    fn handle(
        &mut self,
        action: &Action,
        state: &AppState,
        dispatcher: &Dispatcher,
    ) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            match action {
                // User wants to merge PRs - start async operation
                Action::MergeSelectedPrs => {
                    let selected_prs = get_selected_prs(state);
                    let repo = state.repos.recent_repos[state.repos.selected_repo].clone();
                    let octocrab = self.octocrab.clone().unwrap();
                    let dispatcher = dispatcher.clone();

                    // Spawn async task
                    tokio::spawn(async move {
                        // Dispatch loading indicator
                        dispatcher.dispatch(Action::SetTaskStatus(
                            Some(TaskStatus::running("Merging PRs..."))
                        ));

                        // Perform merge
                        let mut success = true;
                        for pr in &selected_prs {
                            if let Err(e) = merge_pr(&octocrab, &repo, pr).await {
                                success = false;
                                log::error!("Failed to merge PR #{}: {}", pr.number, e);
                            }
                        }

                        // Dispatch result
                        if success {
                            dispatcher.dispatch(Action::MergeComplete(Ok(())));
                            dispatcher.dispatch(Action::ReloadCurrentRepo);
                        } else {
                            dispatcher.dispatch(Action::MergeComplete(
                                Err("Some PRs failed to merge".to_string())
                            ));
                        }
                    });

                    // Let action through to reducer (to update UI state immediately)
                    true
                }

                // User wants to load repo
                Action::LoadRepo(repo_index) => {
                    let repo = state.repos.recent_repos[*repo_index].clone();
                    let octocrab = self.octocrab.clone().unwrap();
                    let cache = self.cache.clone();
                    let dispatcher = dispatcher.clone();

                    tokio::spawn(async move {
                        match load_repo_data(&octocrab, &cache, &repo).await {
                            Ok(prs) => {
                                dispatcher.dispatch(Action::RepoDataLoaded(
                                    *repo_index,
                                    Ok(prs)
                                ));
                            }
                            Err(e) => {
                                dispatcher.dispatch(Action::RepoDataLoaded(
                                    *repo_index,
                                    Err(e.to_string())
                                ));
                            }
                        }
                    });

                    true
                }

                // All other actions pass through
                _ => true,
            }
        })
    }
}
```

### 5. Example Middleware: Logging

```rust
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn handle(
        &mut self,
        action: &Action,
        _state: &AppState,
        _dispatcher: &Dispatcher,
    ) -> BoxFuture<'_, bool> {
        Box::pin(async move {
            log::debug!("Action: {:?}", action);
            true // Always continue
        })
    }
}
```

### 6. Simplified Main Loop

**Before** (complex):
```rust
async fn update(app: &mut App, msg: Action) -> Result<Action> {
    // Filter actions based on UI state
    let msg = if app.store.state().ui.close_pr_state.is_some() { ... };

    // Dispatch to reducer, get effects
    let effects = app.store.dispatch(msg);

    // Execute effects, which return more actions
    for effect in effects {
        let follow_up_actions = execute_effect(app, effect).await?;
        for action in follow_up_actions {
            // Dispatch follow-up actions, creating recursion
            let nested_effects = app.store.dispatch(action);
            for nested_effect in nested_effects {
                let nested_actions = execute_effect(app, nested_effect).await?;
                for nested_action in nested_actions {
                    let _ = app.action_tx.send(nested_action);
                }
            }
        }
    }
    Ok(Action::None)
}
```

**After** (simple):
```rust
async fn update(app: &mut App, action: Action) {
    // Just dispatch - middleware handles everything
    app.store.dispatch(action).await;
}

// Main loop
loop {
    tokio::select! {
        Some(action) = action_rx.recv() => {
            update(&mut app, action).await;
        }
        Some(event) = event_rx.recv() => {
            if let Some(action) = handle_event(event, &app.store.state()) {
                update(&mut app, action).await;
            }
        }
    }
}
```

---

## Benefits

### 1. **Clearer Data Flow**

```
Action → Middleware Chain → Reducer → State
```

No hidden recursion, no effect chaining, traceable.

### 2. **Testable Components**

```rust
#[test]
fn test_reducer_pure() {
    let state = AppState::default();
    let action = Action::RepoDataLoaded(0, Ok(vec![pr1, pr2]));

    let new_state = reduce(state, &action);

    assert_eq!(new_state.repos.repo_data[0].prs.len(), 2);
    // No effects to verify, no side effects
}

#[test]
async fn test_middleware() {
    let middleware = TaskMiddleware::new(octocrab, cache);
    let (tx, rx) = mpsc::unbounded_channel();
    let dispatcher = Dispatcher { tx };

    let should_continue = middleware.handle(
        &Action::MergeSelectedPrs,
        &state,
        &dispatcher,
    ).await;

    assert!(should_continue);
    // Can verify async actions dispatched through rx
}
```

### 3. **Simpler Mental Model**

- **Reducer**: Pure state transformation
- **Middleware**: Side effects and async operations
- **Action**: Unified message type
- **Store**: Orchestrates the flow

### 4. **Better Performance**

- No Vec<Effect> allocations
- No recursive dispatch loops
- Single action queue
- Middleware can optimize (caching, batching)

### 5. **Easier Debugging**

```rust
// Add debug middleware easily:
store.add_middleware(LoggingMiddleware);
store.add_middleware(PerformanceMiddleware);
store.add_middleware(ActionReplayMiddleware);
```

---

## Migration Strategy

### Phase 1: Add Middleware System (No Breaking Changes)

1. Create `middleware.rs` with trait and Dispatcher
2. Update Store to support middleware chain
3. Keep existing Effect system running in parallel
4. Add "bridge middleware" that converts actions to effects

```rust
struct BridgeMiddleware;

impl Middleware for BridgeMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher)
        -> BoxFuture<'_, bool>
    {
        // Convert specific actions to effects for backward compatibility
        match action {
            Action::MergeSelectedPrs => {
                // Old system: would return Effect::PerformMerge
                // New system: middleware handles it directly
            }
            _ => {}
        }
        Box::pin(async { true })
    }
}
```

### Phase 2: Port Effects to Middleware (One at a Time)

1. Start with simplest effects (LoadEnvFile, InitializeOctocrab)
2. Create corresponding middleware handlers
3. Remove effect handlers from execute_effect
4. Test thoroughly

Example:
```rust
// Old: Effect::LoadEnvFile → execute_effect → loads file
// New: Action::Bootstrap → BootstrapMiddleware → loads file
```

### Phase 3: Remove Effect System

1. Once all effects ported, remove Effect enum
2. Update reducers to return only State
3. Remove execute_effect function
4. Update all tests

### Phase 4: Simplify Actions (Optional)

Consider splitting Action enum:
```rust
// User-initiated actions (from keyboard/mouse)
pub enum UserAction {
    Quit,
    Rebase,
    MergeSelectedPrs,
    // ... ~40 actions
}

// System actions (from middleware/async tasks)
pub enum SystemAction {
    RepoDataLoaded(usize, Result<Vec<Pr>, String>),
    MergeComplete(Result<(), String>),
    // ... ~60 actions
}

// Unified Action
pub enum Action {
    User(UserAction),
    System(SystemAction),
}
```

---

## Example: Complete Flow Comparison

### Before (Current System)

```
User presses 'm' (merge)
  ↓
handle_key_event creates Action::MergeSelectedPrs
  ↓
update() dispatches to store
  ↓
repos_reducer matches Action::MergeSelectedPrs
  ├─ Updates state (sets loading indicator)
  ├─ Returns Effect::DispatchAction(Action::StartOperationMonitor)
  ├─ Returns Effect::StartOperationMonitoring
  └─ Returns Effect::PerformMerge
  ↓
update() executes Effect::PerformMerge
  ├─ execute_effect sends BackgroundTask::Merge to task channel
  ├─ Returns Action::SetTaskStatus
  ↓
update() dispatches Action::SetTaskStatus
  ├─ task_reducer updates status
  └─ Returns no effects
  ↓
BackgroundTask::Merge processes in parallel
  ├─ Calls GitHub API
  ├─ Sends TaskResult::MergeComplete to result channel
  ↓
result_to_action converts to Action::MergeComplete
  ↓
update() dispatches Action::MergeComplete
  ↓
repos_reducer matches Action::MergeComplete
  ├─ Clears selection
  ├─ Returns Effect::LoadSingleRepo
  └─ Returns Effect::SetTaskStatus
  ↓
(continues with more effects...)
```

**Total steps: 15+, 3 channels, 2 enum conversions**

### After (Middleware System)

```
User presses 'm' (merge)
  ↓
handle_key_event creates Action::MergeSelectedPrs
  ↓
store.dispatch(Action::MergeSelectedPrs)
  ↓
TaskMiddleware.handle()
  ├─ Spawns async task: merge_prs()
  ├─ Returns true (let action through)
  ↓
reduce() transforms state
  ├─ Sets loading indicator
  └─ Returns new state
  ↓
merge_prs() async task
  ├─ Calls GitHub API
  ├─ Dispatches Action::MergeComplete(result)
  ↓
store.dispatch(Action::MergeComplete)
  ↓
reduce() transforms state
  ├─ Clears selection
  ├─ Updates status
  └─ Returns new state
  ↓
TaskMiddleware.handle() sees MergeComplete
  ├─ Dispatches Action::ReloadCurrentRepo
  └─ Returns true
  ↓
reduce() handles reload
  └─ Sets loading state
  ↓
TaskMiddleware sees reload request
  └─ Spawns load_repo_data()
```

**Total steps: 8, 1 channel, 0 enum conversions**

---

## Open Questions

1. **Middleware Ordering**: Does order matter? (Probably yes - logging should be first)
2. **Middleware State**: Should middleware have their own state? (TaskMiddleware needs Octocrab)
3. **Middleware Lifecycle**: When to initialize/cleanup? (During Store creation)
4. **Error Handling**: How do middleware report errors? (Dispatch error actions)
5. **Cancellation**: How to cancel long-running middleware tasks? (Add CancelToken to Dispatcher)

---

## Recommendation

**Start with Phase 1**: Add middleware system alongside current architecture. This allows:
- Gradual migration
- Testing new system in parallel
- Easy rollback if issues found
- Learning curve for team

Target: Port 1-2 simple effects per week until complete.

---

## Additional Resources

- [Redux Middleware Concept](https://redux.js.org/understanding/history-and-design/middleware)
- [Elm Effects Pattern](https://guide.elm-lang.org/effects/)
- [Rust async-trait crate](https://docs.rs/async-trait/latest/async_trait/)
- [Tokio Channels](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)
