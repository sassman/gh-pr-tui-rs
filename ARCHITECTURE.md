# Architecture

This document describes the architectural patterns and data flow in gh-pr-lander.

## Overview

The application follows a Redux-inspired architecture with a separation between:
- **Main thread**: Rendering and user input handling
- **Background thread**: Middleware processing (API calls, file I/O, async operations)

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              MAIN THREAD                                    │
│                                                                             │
│  ┌──────────┐    action_tx    ┌─────────────┐    result_rx    ┌──────────┐  │
│  │  User    │ ───────────────►│             │◄────────────────│  Store   │  │
│  │  Input   │                 │             │                 │(Reducers)│  │
│  └──────────┘                 │             │                 └──────────┘  │
│                               │             │                       │       │
│       ┌───────────────────────│  Channels   │                       ▼       │
│       │  Action::Event        │             │                 ┌──────────┐  │
│       │  re-routes here       │             │                 │  Render  │  │
│       │                       │             │                 └──────────┘  │
└───────┼───────────────────────┼─────────────┼───────────────────────────────┘
        │                       │             │
        │                       │  action_rx  │  result_tx
        │                       │             │
┌───────┼───────────────────────┼─────────────┼───────────────────────────────┐
│       │                       ▼             │      BACKGROUND THREAD        │
│       │                                     │                               │
│  ┌────┴───────────────────────────────────────────────────────────────────┐ │
│  │                       Middleware Chain                                 │ │
│  │                                                                        │ │
│  │  ┌────────────┐    ┌────────────┐    ┌────────────┐                    │ │
│  │  │ Middleware │───►│ Middleware │───►│ Middleware │───► result_tx      │ │
│  │  └────────────┘    └────────────┘    └────────────┘                    │ │
│  │        │                 │                 │                           │ │
│  │        └─────────────────┴─────────────────┴───► Dispatcher            │ │
│  │                                                      │                 │ │
│  │                                                      ▼                 │ │
│  │                                              action_tx (re-entry)      │ │
│  │                                                                        │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Dispatcher Pattern

Middleware uses a `Dispatcher` to dispatch actions that re-enter the middleware chain.
This enables patterns like:
- `Event::ClientReady` → triggers `LoadRecentRepositories`
- `LoadRecentRepositories` → flows through middleware → handled by `RepositoryMiddleware`

```rust
// Dispatcher wraps action_tx for middleware re-entry
pub struct Dispatcher {
    action_tx: Sender<Action>,
}

impl Dispatcher {
    pub fn dispatch(&self, action: Action) {
        self.action_tx.send(action).ok();
    }
}
```

**Key insight**: Actions dispatched via `Dispatcher` re-enter the middleware chain from the
beginning, ensuring all middleware can observe and react to them.

## Commands vs Events

Actions are categorized into two types:

| Type | Intent | Naming | Dispatch Method |
|------|--------|--------|-----------------|
| **Command** | "Do this" (imperative) | Imperative verb | `dispatcher.dispatch(...)` |
| **Event** | "This happened" (fact) | Past tense | `dispatcher.dispatch(Action::event(...))` |

### Events (`src/actions/event.rs`)

Events represent facts/observations that middleware should be able to observe and react to.
They use the `Action::Event(Event)` variant and re-enter the middleware chain via Dispatcher.

```rust
// Sending an event (re-enters middleware chain)
dispatcher.dispatch(Action::event(Event::ClientReady));

// Handling an event in middleware
Action::Event(Event::ClientReady) => {
    // React to the event
    dispatcher.dispatch(Action::Bootstrap(BootstrapAction::LoadRepositories));
    true
}
```

**Rule: Events MUST use `Action::event(Event::X)` to ensure they re-enter middleware.**

Events cannot be created any other way - the `Event` enum is separate from action enums,
making misuse impossible at compile time.

### Commands

Commands are action requests. When dispatched via `Dispatcher`, they re-enter middleware
and eventually flow to reducers for state updates.

```rust
// Sending a command (re-enters middleware, then goes to reducers)
dispatcher.dispatch(Action::Bootstrap(BootstrapAction::LoadRepositories));
```

## Action Design Principles

### Actions Carry Domain Data, Not Indices

Actions carry domain model data (e.g., `Repository`) instead of state indices (e.g., `repo_idx: usize`).
This avoids **state timing issues** where middleware-dispatched actions might see stale state.

**Problem with indices:**
```rust
// Middleware A dispatches action
dispatcher.dispatch(PullRequestAction::LoadStart(repo_idx));
// Middleware B handles it, but reducer hasn't run yet!
// State lookup with repo_idx might fail or return wrong data
```

**Solution with domain data:**
```rust
// Actions carry the data they need
PullRequestAction::LoadStart { repo: Repository }
PullRequestAction::Loaded { repo: Repository, prs: Vec<Pr> }

// Reducer looks up index when processing
fn find_repo_idx(state: &MainViewState, repo: &Repository) -> Option<usize> {
    state.repositories.iter()
        .position(|r| r.org == repo.org && r.repo == repo.repo)
}
```

This ensures actions are self-contained and can be processed regardless of when the reducer runs.

### Bulk Loading Coordination

`RepositoryMiddleware` coordinates bulk repository loading:
1. Tracks pending repos in `HashSet<Repository>` when `LoadRecentRepositories` is handled
2. Listens for `PullRequestAction::Loaded` / `LoadError` to mark repos as done
3. Dispatches `LoadRecentRepositoriesDone` when all repos complete

This pattern keeps loading orchestration in one place while actual API calls remain in `GitHubMiddleware`.

## Components

### Actions (`src/actions/`)

Actions represent intentions or facts that flow through the system. They are organized by domain:

- `Event` - Facts/observations that re-enter middleware chain
- `GlobalAction` - Application-wide actions (Quit, KeyPressed, Tick)
- `BootstrapAction` - Initialization sequence commands
- `PullRequestAction` - PR-related operations
- `RepositoryAction` - Repository management
- etc.

### Middleware (`src/middleware/`)

Middleware intercepts actions before they reach reducers. Each middleware can:
- Handle the action and produce side effects (API calls, file I/O)
- Dispatch new actions via `Dispatcher` (re-enters middleware chain)
- Pass the action through to the next middleware (return `true`)
- Consume the action, stopping the chain (return `false`)

```rust
pub trait Middleware: Send {
    fn handle(
        &mut self,
        action: &Action,
        state: &AppState,
        dispatcher: &Dispatcher,
    ) -> bool;
}
```

**Middleware order matters** - they are processed sequentially:
1. BootstrapMiddleware
2. AppConfigMiddleware
3. GitHubMiddleware
4. KeyboardMiddleware
5. NavigationMiddleware
6. TextInputMiddleware
7. CommandPaletteMiddleware
8. ConfirmationPopupMiddleware
9. RepositoryMiddleware
10. PullRequestMiddleware
11. DebugConsoleMiddleware

### Reducers (`src/reducers/`)

Reducers are pure functions that update state based on actions. They receive actions from
the background thread (via `result_tx`) after middleware processing.

### Store (`src/store.rs`)

The store holds the application state and dispatches actions to reducers.

## Action Flow Details

### Regular Actions (Commands)
```
User Input → action_tx → middleware chain
                              │
                              ├──► Dispatcher.dispatch() ──► action_tx (re-entry)
                              │
                              └──► result_tx (if not consumed) ──► reducers
```

### Events
```
Middleware dispatches Event → Dispatcher → action_tx
                                              ↓
                                    middleware chain (all middleware see it)
                                              ↓
                                    (Events are NOT forwarded to reducers)
```

**Important:** Events are consumed after middleware processing. They do NOT flow to
reducers, preventing infinite loops.

## State Management

- `AppState` is the single source of truth
- Main thread owns the `Store` and applies reducer updates
- Background thread reads state via `SharedState` (Arc<RwLock<AppState>>)
- State is synced to `SharedState` after each reducer dispatch

## Async Operations

Some middleware (like `GitHubMiddleware`) maintains its own Tokio runtime for async operations:
- API calls to GitHub
- File I/O operations
- Spawned tasks use cloned `Dispatcher` to send results back through middleware

## File Structure

```
crates/gh-pr-lander/src/
├── main.rs              # Entry point, main loop, terminal setup
├── background.rs        # Background worker thread
├── dispatcher.rs        # Dispatcher for middleware action dispatch
├── store.rs             # Store implementation
├── state.rs             # AppState definition
├── actions/             # Action enum definitions
│   ├── event.rs         # Event enum (facts that re-enter middleware)
│   └── ...              # Domain-specific action enums
├── middleware/          # Middleware implementations
├── reducers/            # Reducer functions
├── views/               # UI rendering (ratatui)
├── keybindings/         # Key binding configuration
└── domain_models/       # Domain types (PR, Repository, etc.)
```
