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
    debug_console_reducer, key_bindings_reducer, pull_request_reducer, splash_reducer,
    status_bar_reducer,
};
use crate::state::AppState;
use crate::views::MainView;

/// Reducer - pure function that produces new state from current state + action
///
/// This is the root reducer that orchestrates all sub-reducers.
/// It routes actions by their tag (screen-specific variants) or translates
/// generic actions via the active view.
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    match action {
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
        // GENERIC ACTIONS - Handled by middleware (translation to view-specific actions)
        // These reach the reducer only if no view handles them
        // =======================================================================
        Action::Navigate(_) | Action::TextInput(_) => {
            // Translation is handled by NavigationMiddleware and TextInputMiddleware
            // If we reach here, the action was not handled by any view
            state
        }

        // =======================================================================
        // SCREEN-SPECIFIC ACTIONS - Route by tag to type-safe reducers
        // =======================================================================
        Action::PullRequest(sub) => {
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
                    let repo = state.add_repo_form.to_repository();
                    log::info!("Adding repository: {}", repo.display_name());
                    state.main_view.repositories.push(repo);
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
                }
                BootstrapAction::End => {
                    state.splash.bootstrapping = false;
                    state.view_stack.clear();
                    state.view_stack.push(Box::new(MainView::new()));
                }
                BootstrapAction::ConfigLoaded(config) => {
                    state.app_config = config.clone();
                    log::info!("App config loaded into state");
                }
                BootstrapAction::LoadRecentRepositories
                | BootstrapAction::LoadRecentRepositoriesDone => {
                    // Handled by middleware
                }
                BootstrapAction::RepositoryAddBulk(repos) => {
                    let count = repos.len();
                    log::info!("Adding {} repositories from config", count);
                    state.main_view.repositories.extend(repos.clone());
                }
            }
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

        // No-op action
        Action::None => state,
    }
}
