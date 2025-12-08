//! App Reducer - Root reducer that orchestrates all sub-reducers
//!
//! This reducer uses a tagged action architecture:
//! - Generic actions (`Navigate`, `TextInput`) are translated by the active view
//! - Screen-specific actions are routed directly by their tag
//! - No ViewId matching - views own their action translation

use crate::actions::{
    Action, AddRepositoryAction, BootstrapAction, CommandPaletteAction, GlobalAction,
    KeyBindingsAction,
};
use crate::reducers::{
    add_repo_reducer, build_log_reducer, command_palette_reducer, confirmation_popup_reducer,
    debug_console_reducer, diff_viewer_reducer, key_bindings_reducer, pull_request_reducer,
    session_reducer, splash_reducer, status_bar_reducer,
};
use crate::state::AppState;
use crate::views::DiffViewerView;

/// Reducer - pure function that produces new state from current state + action
///
/// This is the root reducer that orchestrates all sub-reducers.
/// It routes actions by their tag (screen-specific variants) or translates
/// generic actions via the active view.
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    match action {
        // =======================================================================
        // EVENTS - Re-routed to middleware in main loop, should not reach here
        // =======================================================================
        Action::Event(_) => {
            // Events are re-routed to middleware chain in main.rs
            // If we reach here, it's unexpected but harmless - just pass through
            log::trace!("Event reached reducer (unexpected): {:?}", action);
            state
        }

        // =======================================================================
        // GLOBAL ACTIONS - Application-wide behavior
        // =======================================================================
        Action::Global(GlobalAction::Quit) => {
            state.running = false;
            state
        }

        Action::Global(GlobalAction::Close) => {
            if state.view_stack.len() > 1 {
                let popped = state.view_stack.pop();
                log::debug!("Closed view: {:?}", popped.map(|v| v.view_id()));
            } else {
                log::debug!("Closing last view - quitting application");
                state.running = false;
            }
            state
        }

        Action::Global(GlobalAction::PushView(new_view)) => {
            // Check if this view is already the top-most view (toggle behavior)
            let is_duplicate = state
                .view_stack
                .last()
                .map(|top| top.view_id() == new_view.view_id())
                .unwrap_or(false);

            if is_duplicate {
                log::debug!(
                    "Popping view from the stack, because this view is on top already: {:?}",
                    new_view.view_id()
                );
                state.view_stack.pop();
            } else {
                log::debug!("Pushing view onto stack: {:?}", new_view.view_id());
                state.view_stack.push(new_view.clone());
            }
            state
        }

        Action::Global(GlobalAction::ReplaceView(new_view)) => {
            log::debug!("Replacing view stack with: {:?}", new_view.view_id());
            state.view_stack.clear();
            state.view_stack.push(new_view.clone());
            state
        }

        Action::Global(GlobalAction::Tick) => {
            // Tick is used for animations - delegate to splash reducer
            state.splash =
                splash_reducer::reduce_splash(state.splash, &crate::actions::SplashAction::Tick);
            state
        }

        Action::Global(GlobalAction::KeyPressed(_)) => {
            // Handled by keyboard middleware, not by reducer
            state
        }

        // =======================================================================
        // GENERIC ACTIONS - Translate via active view and recurse
        // =======================================================================
        Action::Navigate(nav) => {
            if let Some(view) = state.view_stack.last() {
                if let Some(translated) = view.translate_navigation(*nav) {
                    // Recurse with the translated action
                    return reduce(state, &translated);
                }
            }
            log::debug!("Navigation action not handled by active view: {:?}", nav);
            state
        }

        Action::TextInput(input) => {
            if let Some(view) = state.view_stack.last() {
                if let Some(translated) = view.translate_text_input(input.clone()) {
                    // Recurse with the translated action
                    return reduce(state, &translated);
                }
            }
            log::debug!("TextInput action not handled by active view: {:?}", input);
            state
        }

        Action::ViewContext(ctx_action) => {
            if let Some(view) = state.view_stack.last() {
                if let Some(translated) = view.translate_context_action(*ctx_action, &state) {
                    // Recurse with the translated view-specific action
                    return reduce(state, &translated);
                }
            }
            log::debug!(
                "ViewContext action not handled by active view: {:?}",
                ctx_action
            );
            state
        }

        // =======================================================================
        // SCREEN-SPECIFIC ACTIONS - Route by tag to type-safe reducers
        // =======================================================================
        Action::PullRequest(sub) => {
            // TODO: here we should have a dedicated pull request state in the future
            state.main_view = pull_request_reducer::reduce_pull_request(state.main_view, sub);
            state
        }

        Action::CommandPalette(sub) => {
            // Handle Close and Execute here for view stack management
            if matches!(
                sub,
                CommandPaletteAction::Close | CommandPaletteAction::Execute
            ) && state.view_stack.len() > 1
            {
                let popped = state.view_stack.pop();
                log::debug!("Closed view: {:?}", popped.map(|v| v.view_id()));
            }
            state.command_palette = command_palette_reducer::reduce_command_palette(
                state.command_palette,
                sub,
                &state.keymap,
            );
            state
        }

        Action::AddRepository(sub) => {
            // Handle Close here for form reset and view stack management
            if matches!(sub, AddRepositoryAction::Close) {
                state.add_repo_form.reset();
                if state.view_stack.len() > 1 {
                    state.view_stack.pop();
                }
            }
            // Handle Confirm - add repository if valid
            if matches!(sub, AddRepositoryAction::Confirm) {
                if state.add_repo_form.is_valid() {
                    state.add_repo_form.reset();
                    if state.view_stack.len() > 1 {
                        state.view_stack.pop();
                    }
                } else {
                    log::warn!(
                        "Cannot add repository: form is not valid (org and repo are required)"
                    );
                }
            }
            state.add_repo_form = add_repo_reducer::reduce_add_repository(state.add_repo_form, sub);
            state
        }

        Action::KeyBindings(sub) => {
            // Handle Close here for view stack management
            if matches!(sub, KeyBindingsAction::Close) && state.view_stack.len() > 1 {
                let popped = state.view_stack.pop();
                log::debug!("Closed view: {:?}", popped.map(|v| v.view_id()));
            }
            state.key_bindings_panel =
                key_bindings_reducer::reduce_key_bindings(state.key_bindings_panel, sub);
            state
        }

        Action::DebugConsole(sub) => {
            state.debug_console =
                debug_console_reducer::reduce_debug_console(state.debug_console, sub);
            state
        }

        Action::Splash(sub) => {
            state.splash = splash_reducer::reduce_splash(state.splash, sub);
            state
        }

        Action::Bootstrap(sub) => {
            match sub {
                BootstrapAction::Start => {
                    state.splash.bootstrapping = true;
                    state.splash.animation_frame = 0;
                    state.splash.loading_complete = false;
                }
                BootstrapAction::End => {
                    // Mark loading as complete, but only transition if min duration elapsed
                    state.splash.loading_complete = true;
                    state.splash.bootstrapping = false;
                }
                BootstrapAction::ConfigLoaded(config) => {
                    state.app_config = config.clone();
                    log::info!("App config loaded into state");
                }
                BootstrapAction::LoadRecentRepositories
                | BootstrapAction::LoadRecentRepositoriesDone => {
                    // Handled by middleware
                }
            }
            state
        }

        // Session actions - delegate to session reducer
        Action::Session(sub) => {
            state.main_view = session_reducer::reduce_session(state.main_view, sub);
            state
        }

        // MergeBot actions - currently handled by middlewares (no state changes in reducer)
        Action::MergeBot(_) => state,

        // Status bar actions
        Action::StatusBar(sub) => {
            state.status_bar = status_bar_reducer::reduce_status_bar(state.status_bar, sub);
            state
        }

        // Build log actions
        Action::BuildLog(sub) => {
            state.build_log = build_log_reducer::reduce_build_log(state.build_log, sub);
            state
        }

        // Confirmation popup actions - delegate to dedicated reducer
        Action::ConfirmationPopup(sub) => {
            confirmation_popup_reducer::reduce_confirmation_popup(state, sub)
        }

        // Diff viewer actions
        Action::DiffViewer(sub) => {
            use crate::actions::DiffViewerAction;

            // Handle Open specially to push view onto stack
            if matches!(sub, DiffViewerAction::Open) {
                log::debug!("Opening diff viewer");
                state.view_stack.push(Box::new(DiffViewerView::new()));
            }

            state.diff_viewer = diff_viewer_reducer::reduce_diff_viewer(state.diff_viewer, sub);
            state
        }

        Action::Repository(sub) => {
            // Handled by repository reducer
            state.main_view =
                crate::reducers::repository_reducer::reduce_repository(state.main_view, sub);
            state
        }

        // No-op action
        Action::None => state,
    }
}
