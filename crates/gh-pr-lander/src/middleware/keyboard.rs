use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::{actions::Action, state::ActiveView};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// KeyboardMiddleware - converts raw keyboard events to semantic actions
pub struct KeyboardMiddleware;

impl KeyboardMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for KeyboardMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        if let Action::GlobalKeyPressed(key) = action {
            handle_key_event(key, state, dispatcher);
            // Consume the raw key event (don't pass to reducer)
            return false;
        }

        // Pass all other actions through
        true
    }
}

/// Handle a key event and dispatch semantic actions
fn handle_key_event(key: &KeyEvent, state: &AppState, dispatcher: &Dispatcher) {
    match key.code {
        // Global close/quit
        KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::GlobalQuit);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            dispatcher.dispatch(Action::GlobalQuit);
        }
        KeyCode::Esc => {
            dispatcher.dispatch(Action::GlobalClose);
        }

        // Vim navigation - down/next
        KeyCode::Char('j') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavNext);
        }
        KeyCode::Down => {
            dispatcher.dispatch(Action::NavNext);
        }

        // Vim navigation - up/previous
        KeyCode::Char('k') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavPrevious);
        }
        KeyCode::Up => {
            dispatcher.dispatch(Action::NavPrevious);
        }

        // Vim navigation - left
        KeyCode::Char('h') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavLeft);
        }
        KeyCode::Left => {
            dispatcher.dispatch(Action::NavLeft);
        }

        // Vim navigation - right
        KeyCode::Char('l') if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::NavRight);
        }
        KeyCode::Right => {
            dispatcher.dispatch(Action::NavRight);
        }

        // Vim navigation - jump to end
        KeyCode::Char('G') if key.modifiers == KeyModifiers::SHIFT => {
            dispatcher.dispatch(Action::NavJumpToEnd);
        }

        // TODO: ActiveView changing seems to require some state knowledge at least, there must be a better way..
        KeyCode::Char('`') if key.modifiers == KeyModifiers::NONE => {
            let same_view = ActiveView::DebugConsole;
            if state.active_view != same_view {
                dispatcher.dispatch(Action::GlobalActivateView(same_view));
            } else {
                dispatcher.dispatch(Action::GlobalActivateView(ActiveView::Main));
            }
        }

        // Any other character key without modifiers - dispatch as LocalKeyPressed
        KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
            dispatcher.dispatch(Action::LocalKeyPressed(c));
        }

        // Unhandled keys
        _ => {
            log::trace!("Unhandled key: {:?}", key);
        }
    }
}
