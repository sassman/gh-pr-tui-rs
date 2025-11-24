# Capability-Based Vim Scrolling System

## Overview

We've implemented a capability-based system for vim-style scrolling that separates keybinding logic from panel logic and makes scrolling behavior data-driven rather than hardcoded.

## What We've Implemented (Steps 1-7 from the Plan)

### 1. ✅ Scroll Capability Data Type (`capabilities.rs`)

Created `PanelCapabilities` bitflags that panels can declare:

```rust
bitflags! {
    pub struct PanelCapabilities: u32 {
        const SCROLL_VERTICAL = 1 << 0;
        const SCROLL_HORIZONTAL = 1 << 1;
        const VIM_SCROLL_BINDINGS = 1 << 2;
        const VIM_NAVIGATION_BINDINGS = 1 << 3;
        const ITEM_NAVIGATION = 1 << 4;
        const ITEM_SELECTION = 1 << 5;
    }
}
```

Helper methods:
- `supports_vim_vertical_scroll()` - Requires both SCROLL_VERTICAL and VIM_SCROLL_BINDINGS
- `supports_vim_horizontal_scroll()` - Requires both SCROLL_HORIZONTAL and VIM_SCROLL_BINDINGS
- `supports_vim_navigation()` - Checks VIM_NAVIGATION_BINDINGS

### 2. ✅ Capability Provider Trait

```rust
pub trait PanelCapabilityProvider {
    fn capabilities(&self) -> PanelCapabilities;
}
```

Panels implement this trait to declare their capabilities dynamically based on their state.

### 3. ✅ Reflect Capabilities in AppState (`state.rs`)

Added to `UiState`:

```rust
pub struct UiState {
    /// Capabilities of the currently active panel
    pub active_panel_capabilities: PanelCapabilities,
    // ... rest of fields
}
```

Default is PR table capabilities (vim navigation + item navigation + selection).

### 5. ✅ Semantic Scroll Actions (`actions.rs`)

Added capability-based semantic actions:

```rust
pub enum Action {
    // ... existing actions

    // Semantic scroll actions (vim-style, capability-based)
    ScrollToTop,        // vim: gg - scroll to top of current panel
    ScrollToBottom,     // vim: G - scroll to bottom of current panel
    ScrollPageUp,       // Page up in current panel
    ScrollPageDown,     // Page down in current panel
    ScrollHalfPageUp,   // Half page up (vim: Ctrl+u)
    ScrollHalfPageDown, // Half page down (vim: Ctrl+d)
}
```

### 6-7. ✅ Capability-Aware Key Sequence Handler (`middleware/keyboard.rs`)

Updated `KeyboardMiddleware` to:

1. **Read capabilities from state**:
   ```rust
   let capabilities = state.ui.active_panel_capabilities;
   ```

2. **Pass capabilities to key handler**:
   ```rust
   return self.handle_key(*key, context, capabilities, dispatcher);
   ```

3. **Check capabilities before dispatching**:
   ```rust
   fn handle_sequence(
       &mut self,
       sequence: KeySequence,
       capabilities: PanelCapabilities,
       dispatcher: &Dispatcher,
   ) -> bool {
       match sequence {
           KeySequence::GoToTop => {
               if capabilities.supports_vim_vertical_scroll() {
                   dispatcher.dispatch(Action::ScrollToTop);
                   return false; // Block original key event
               } else {
                   return true; // Pass through
               }
           }
           // ... similar for GoToBottom
       }
   }
   ```

## What Still Needs to Be Done (Steps 4, 8-10)

### Step 4: Update Capabilities When Panel Changes (TODO)

Need to add capability computation logic:

1. **When active panel changes**: Update `active_panel_capabilities` in reducer
2. **When panel content changes**: Recompute capabilities if scrollability changes
3. **Implement `PanelCapabilityProvider` for each panel state**

Example for log panel:
```rust
impl PanelCapabilityProvider for LogPanelState {
    fn capabilities(&self) -> PanelCapabilities {
        let mut caps = PanelCapabilities::VIM_SCROLL_BINDINGS | PanelCapabilities::VIM_NAVIGATION_BINDINGS;

        // Check if content is larger than viewport
        if self.has_vertical_overflow() {
            caps |= PanelCapabilities::SCROLL_VERTICAL;
        }
        if self.has_horizontal_overflow() {
            caps |= PanelCapabilities::SCROLL_HORIZONTAL;
        }

        caps
    }
}
```

### Step 8: Panel Reducers Handle Semantic Scroll Actions (TODO)

Each panel reducer needs to handle the semantic scroll actions:

```rust
// In log panel reducer
match action {
    Action::ScrollToTop => {
        state.vertical_scroll = 0;
    }
    Action::ScrollToBottom => {
        state.vertical_scroll = state.max_vertical_scroll();
    }
    Action::ScrollPageDown => {
        state.vertical_scroll = (state.vertical_scroll + state.viewport_height)
            .min(state.max_vertical_scroll());
    }
    // ... etc
}
```

Panels that don't support scrolling simply ignore these actions.

### Step 9: Wire Focus Updates to Capabilities (TODO)

When focus changes or panel state changes:

```rust
// In reducer when switching panels
Action::ShowLogPanel => {
    // ... other state updates

    // Update capabilities for log panel
    state.ui.active_panel_capabilities = state.log_panel.capabilities();
}
```

### Step 10: Tests (TODO)

Add tests:

1. **Capability tests**: Verify bitflag logic
2. **Keybinding tests**: Given capabilities, verify correct actions dispatched
3. **Reducer tests**: Verify scroll actions update state correctly

## Benefits of This Approach

1. **Decoupled**: Keybindings don't know about specific panels
2. **Data-driven**: Behavior controlled by state, not hardcoded logic
3. **Testable**: Can test keybindings and reducers independently
4. **Debuggable**: Capabilities visible in state, can see why keys don't work
5. **Extensible**: Easy to add new capabilities or panels
6. **Replay-friendly**: State + keys = deterministic behavior

## Example Flow

```
User presses 'G' in log panel
  ↓
KeyPressed(G) action dispatched
  ↓
KeyboardMiddleware intercepts
  ↓
Reads state.ui.active_panel_capabilities
  ↓
Checks: capabilities.supports_vim_vertical_scroll()
  ↓
If true: dispatcher.dispatch(Action::ScrollToBottom)
  ↓
Log panel reducer handles ScrollToBottom
  ↓
Sets scroll position to bottom
  ↓
UI re-renders with new scroll position
```

## Migration Strategy

The current implementation is backwards compatible:

- Old context-based j/k/h/l navigation still works
- New capability-based G/gg scrolling works alongside it
- Gradually migrate other keys to capability-based system
- Eventually remove context entirely

## Next Steps

1. Implement `PanelCapabilityProvider` for each panel state
2. Add capability updates in reducers when panel state changes
3. Implement semantic scroll action handlers in panel reducers
4. Add tests
5. Gradually migrate remaining keys (j/k/h/l) to capability system
6. Remove old context-based logic once migration complete
