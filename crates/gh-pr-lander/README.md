# gh-pr-lander

A clean, minimal PR landing tool built with a Redux-style architecture.

## Architecture

This crate demonstrates a clean, maintainable architecture for TUI applications:

### Core Principles

1. **No God Objects** - Each module has a single, well-defined responsibility
2. **Unidirectional Data Flow** - Actions → Middleware → Reducer → State → View
3. **Prefixed Actions** - Actions are scoped (e.g., `Global`, `Nav`) to indicate their domain
4. **Separation of Concerns** - Business logic separate from presentation

### Directory Structure

```
src/
├── main.rs              # Application entry point, terminal setup
├── actions.rs           # Action definitions (prefixed by scope)
├── state.rs             # Application state
├── reducer.rs           # Pure state transformation logic
├── dispatcher.rs        # Action dispatch mechanism
├── store.rs             # State container and middleware orchestration
├── middleware/          # Middleware chain
│   ├── mod.rs          # Middleware trait
│   ├── logging.rs      # Logs all actions
│   └── keyboard.rs     # Converts keyboard events to semantic actions
├── view_models/         # Pre-computed presentation data (future)
└── views/              # UI rendering
    ├── mod.rs          # View orchestration
    └── main_view.rs    # Main view component
```

### Data Flow

```
User Input (KeyEvent)
  ↓
GlobalKeyPressed(KeyEvent)  [Action dispatched to Store]
  ↓
LoggingMiddleware           [Logs the action]
  ↓
KeyboardMiddleware          [Converts to semantic action]
  ↓
NavNext/NavPrevious/etc     [Semantic action dispatched]
  ↓
Reducer                     [Updates state]
  ↓
View                        [Renders new state]
```

### Key Components

#### Actions (`actions.rs`)
- All actions are prefixed by scope:
  - `Global*` - Not tied to any specific view (e.g., `GlobalClose`, `GlobalQuit`)
  - `Nav*` - Navigation actions (e.g., `NavNext`, `NavPrevious`)
- Actions are pure data - no logic

#### Middleware (`middleware/`)
- Intercepts actions before they reach the reducer
- Can dispatch new actions
- Examples:
  - `LoggingMiddleware` - Trivial logging of all actions
  - `KeyboardMiddleware` - Converts `GlobalKeyPressed` to semantic navigation actions

#### Reducer (`reducer.rs`)
- Pure function: `(State, Action) → State`
- No side effects
- Single source of truth for state transitions

#### Store (`store.rs`)
- Holds application state
- Manages middleware chain
- Orchestrates action dispatch flow

### Current Features

- ✅ Clean Redux architecture
- ✅ Modular reducers (root reducer orchestrates sub-reducers)
- ✅ Logging middleware
- ✅ Keyboard middleware with vim navigation (hjkl)
- ✅ Prefixed action naming convention
- ✅ Custom logger (logs to debug console, not stdout)
- ✅ Debug console (Quake-style drop-down)
- ✅ Local key handling (views handle their own keys)
- ✅ Active view tracking
- ✅ Minimal working TUI

### Controls

- `` ` `` - Toggle debug console (Quake-style)
- `c` - Clear debug console (when console is active)
- `j` or `↓` - Navigate down/next
- `k` or `↑` - Navigate up/previous
- `h` or `←` - Navigate left
- `l` or `→` - Navigate right
- `q` or `Esc` - Close/Quit
- `Ctrl+C` - Force quit

## Running

```bash
cargo run -p gh-pr-lander
```

**Note**: Logs are captured in the debug console (toggle with `` ` ``), not stdout.

With debug logging to stderr (for development):

```bash
DEBUG=1 cargo run -p gh-pr-lander
```

When `DEBUG=1` is set, logs are sent to both stderr and the debug console.

## Migration Plan

This crate will gradually receive clean implementations of features from `gh-pr-tui`:

1. ✅ Redux architecture foundation
2. ✅ Logging and keyboard middleware
3. 🔲 Panel stack (for context-aware Close action)
4. 🔲 PR table view
5. 🔲 GitHub integration
6. 🔲 Additional features...

Each migration will follow clean code principles and avoid the architectural issues from the original crate.
