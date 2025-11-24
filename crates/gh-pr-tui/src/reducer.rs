use crate::{actions::Action, state::*};
use log::{debug, error, info};

// MIGRATION: Effect system removed, all side effects in middleware
type Effect = ();

/// Root reducer that delegates to sub-reducers based on action type
/// Pure function: takes state and action, returns (new state, effects to perform)
pub fn reduce(mut state: AppState, action: &Action) -> (AppState, Vec<Effect>) {
    let mut effects = Vec::new();

    // highest priority is the infrastructure setup, then the rest.
    let (infrastructure_state, infrastructure_effects) =
        infrastructure_reducer(state.infra, action);
    state.infra = infrastructure_state;
    effects.extend(infrastructure_effects);

    let (ui_state, ui_effects) = ui_reducer(state.ui, action, &state.theme);
    state.ui = ui_state;
    effects.extend(ui_effects);

    let (repos_state, repos_effects) = repos_reducer(
        state.repos,
        action,
        &state.config,
        &state.theme,
        &state.infra,
    );
    state.repos = repos_state;
    effects.extend(repos_effects);

    let (log_panel_state, log_panel_effects) =
        log_panel_reducer(state.log_panel, action, &state.theme);
    state.log_panel = log_panel_state;
    effects.extend(log_panel_effects);

    let (merge_bot_state, merge_bot_effects) =
        merge_bot_reducer(state.merge_bot, action, &state.repos);
    state.merge_bot = merge_bot_state;
    effects.extend(merge_bot_effects);

    let (task_state, task_effects) = task_reducer(state.task, action);
    state.task = task_state;
    effects.extend(task_effects);

    let (debug_console_state, debug_console_effects) =
        debug_console_reducer(state.debug_console, action, &state.theme);
    state.debug_console = debug_console_state;
    effects.extend(debug_console_effects);

    // Recompute splash screen view model when needed
    // Only recompute for actions that affect bootstrap state or spinner
    match action {
        Action::Bootstrap
        | Action::SetBootstrapState(_)
        | Action::BootstrapComplete(_)
        | Action::OctocrabInitialized(_)
        | Action::TickSpinner => {
            recompute_splash_screen_view_model(&mut state);
        }
        _ => {}
    }

    // MIGRATION COMPLETE: All side effects now handled by middleware
    (state, vec![])
}

/// UI state reducer - handles UI-related actions
fn ui_reducer(
    mut state: UiState,
    action: &Action,
    theme: &crate::theme::Theme,
) -> (UiState, Vec<Effect>) {
    match action {
        // KeyPressed is handled by KeyboardMiddleware - no-op in reducer
        Action::KeyPressed(_) => {}

        Action::Quit => {
            state.should_quit = true;
        }
        Action::TickSpinner => {
            // Increment spinner frame for animation (0-9 cycle)
            state.spinner_frame = (state.spinner_frame + 1) % 10;
        }
        Action::ToggleShortcuts => {
            state.show_shortcuts = !state.show_shortcuts;
            // Recompute view model when shortcuts panel is shown
            if state.show_shortcuts {
                recompute_shortcuts_panel_view_model(&mut state, theme);
            } else {
                state.shortcuts_panel_view_model = None;
            }
        }
        Action::ScrollShortcutsUp => {
            state.shortcuts_scroll = state.shortcuts_scroll.saturating_sub(1);
            // Recompute view model after scroll
            if state.show_shortcuts {
                recompute_shortcuts_panel_view_model(&mut state, theme);
            }
        }
        Action::ScrollShortcutsDown => {
            if state.shortcuts_scroll < state.shortcuts_max_scroll {
                state.shortcuts_scroll += 1;
            }
            // Recompute view model after scroll
            if state.show_shortcuts {
                recompute_shortcuts_panel_view_model(&mut state, theme);
            }
        }
        Action::CloseLogPanel => {
            // Close shortcuts panel first if open
            if state.show_shortcuts {
                state.show_shortcuts = false;
            }
        }
        Action::ShowAddRepoPopup => {
            state.show_add_repo = true;
            state.add_repo_form = AddRepoForm::default();
        }
        Action::HideAddRepoPopup => {
            state.show_add_repo = false;
            state.add_repo_form = AddRepoForm::default();
        }
        Action::AddRepoFormInput(ch) => {
            // Handle paste detection for GitHub URLs
            let input_str = ch.to_string();
            if input_str.contains("github.com") || state.add_repo_form.org.contains("github.com") {
                // Likely a URL paste, try to parse it
                let url_text = format!("{}{}", state.add_repo_form.org, input_str);
                if let Some((org, repo, branch)) = parse_github_url(&url_text) {
                    state.add_repo_form.org = org;
                    state.add_repo_form.repo = repo;
                    state.add_repo_form.branch = branch;
                    return (state, vec![]);
                }
            }

            // Normal character input to current field
            match state.add_repo_form.focused_field {
                AddRepoField::Org => state.add_repo_form.org.push(*ch),
                AddRepoField::Repo => state.add_repo_form.repo.push(*ch),
                AddRepoField::Branch => state.add_repo_form.branch.push(*ch),
            }
        }
        Action::AddRepoFormBackspace => match state.add_repo_form.focused_field {
            AddRepoField::Org => {
                state.add_repo_form.org.pop();
            }
            AddRepoField::Repo => {
                state.add_repo_form.repo.pop();
            }
            AddRepoField::Branch => {
                state.add_repo_form.branch.pop();
            }
        },
        Action::AddRepoFormNextField => {
            state.add_repo_form.focused_field = match state.add_repo_form.focused_field {
                AddRepoField::Org => AddRepoField::Repo,
                AddRepoField::Repo => AddRepoField::Branch,
                AddRepoField::Branch => AddRepoField::Org,
            };
        }
        Action::AddRepoFormSubmit => {
            // MIGRATION NOTE: AddRepository now handled by TaskMiddleware
            // Middleware will:
            // - Check if repo exists
            // - Save to file
            // - Dispatch RepositoryAdded, SelectRepoByIndex, ReloadRepo

            // Just hide the form and reset it
            if !state.add_repo_form.org.is_empty() && !state.add_repo_form.repo.is_empty() {
                state.show_add_repo = false;
                state.add_repo_form = AddRepoForm::default();
            }
        }
        Action::ShowClosePrPopup => {
            state.close_pr_state = Some(crate::state::ClosePrState::new());
        }
        Action::HideClosePrPopup => {
            state.close_pr_state = None;
        }
        Action::ClosePrFormInput(ch) => {
            if let Some(ref mut close_pr) = state.close_pr_state {
                close_pr.comment.push(*ch);
            }
        }
        Action::ClosePrFormBackspace => {
            if let Some(ref mut close_pr) = state.close_pr_state {
                close_pr.comment.pop();
            }
        }
        Action::ClosePrFormSubmit => {
            // MIGRATION NOTE: ClosePrs now handled by TaskMiddleware
            // Middleware will:
            // - Get selected PRs from state
            // - Dispatch SetTaskStatus
            // - Send BackgroundTask::ClosePrs
            // Just keep the state change here
            if let Some(_close_pr) = state.close_pr_state.take() {
                // close_pr_state is cleared, middleware handles the rest
            }
        }

        // Command palette actions
        Action::ShowCommandPalette => {
            state.command_palette = Some(crate::state::CommandPaletteState::new());
            // Trigger filter update to populate initial commands
            return (state, vec![]);
        }
        Action::HideCommandPalette => {
            state.command_palette = None;
        }
        Action::CommandPaletteInput(ch) => {
            if let Some(ref mut palette) = state.command_palette {
                palette.input.push(*ch);
                palette.selected_index = 0; // Reset selection when typing
                return (state, vec![]);
            }
        }
        Action::CommandPaletteBackspace => {
            if let Some(ref mut palette) = state.command_palette {
                palette.input.pop();
                palette.selected_index = 0; // Reset selection when typing
                return (state, vec![]);
            }
        }
        // Cache management actions
        Action::ClearCache => {
            return (state, vec![]);
        }
        Action::ShowCacheStats => {
            return (state, vec![]);
        }
        Action::InvalidateRepoCache(repo_index) => {
            return (state, vec![]);
        }

        // UI management actions
        Action::ForceRedraw => {
            state.force_redraw = true;
        }
        Action::ResetForceRedraw => {
            state.force_redraw = false;
        }
        Action::FatalError(err) => {
            state.should_quit = true;
            // Also set error in loading state for visibility
            // Note: Using repos.loading_state as a general error indicator
        }
        Action::UpdateShortcutsMaxScroll(max_scroll) => {
            state.shortcuts_max_scroll = *max_scroll;
        }

        Action::CommandPaletteSelectNext => {
            if let Some(ref mut palette) = state.command_palette
                && !palette.filtered_commands.is_empty()
            {
                palette.selected_index =
                    (palette.selected_index + 1) % palette.filtered_commands.len();
                // Recompute view model
                recompute_command_palette_view_model(&mut state, theme);
            }
        }
        Action::CommandPaletteSelectPrev => {
            if let Some(ref mut palette) = state.command_palette
                && !palette.filtered_commands.is_empty()
            {
                palette.selected_index = palette
                    .selected_index
                    .checked_sub(1)
                    .unwrap_or(palette.filtered_commands.len() - 1);
                // Recompute view model
                recompute_command_palette_view_model(&mut state, theme);
            }
        }
        Action::CommandPaletteExecute => {
            // Execute the selected command and close palette
            if let Some(palette) = state.command_palette.take()
                && let Some((cmd, _score)) = palette.filtered_commands.get(palette.selected_index)
            {
                // Dispatch the selected action
                return (state, vec![]);
            }
        }
        Action::UpdateCommandPaletteResults(results) => {
            // Update filtered commands in palette state
            if let Some(ref mut palette) = state.command_palette {
                palette.filtered_commands = results.clone();
                // Clamp selected_index to valid range
                if !palette.filtered_commands.is_empty() {
                    palette.selected_index = palette
                        .selected_index
                        .min(palette.filtered_commands.len() - 1);
                } else {
                    palette.selected_index = 0;
                }
                // Recompute view model
                recompute_command_palette_view_model(&mut state, theme);
            }
        }
        _ => {}
    }

    (state, vec![])
}

/// Recompute command palette view model after state changes
fn recompute_command_palette_view_model(state: &mut UiState, theme: &crate::theme::Theme) {
    if let Some(ref mut palette) = state.command_palette {
        // Use reasonable defaults for terminal dimensions
        // Typical terminal: 80x24, popup is 70% width x 60% height
        // After margins/borders: ~10 rows for content
        const VISIBLE_HEIGHT: usize = 10;

        palette.view_model = Some(
            crate::view_models::command_palette::CommandPaletteViewModel::from_state(
                &palette.input,
                palette.selected_index,
                &palette.filtered_commands,
                VISIBLE_HEIGHT,
                theme,
            ),
        );
    }
}

/// Recompute splash screen view model when bootstrap state changes
fn recompute_splash_screen_view_model(state: &mut AppState) {
    // Only show splash screen during bootstrap (before UI is ready)
    use crate::state::BootstrapState;
    let should_show_splash = !matches!(
        state.infra.bootstrap_state,
        BootstrapState::UIReady | BootstrapState::LoadingRemainingRepos | BootstrapState::Completed
    );

    if should_show_splash {
        // Calculate reasonable bar width (assume typical terminal width)
        // Typical terminal: 80 cols, popup: 50 cols, inner: 46 cols after margins
        // Progress bar area: 46 - 10 (for percentage) = 36 chars
        const BAR_WIDTH: usize = 36;

        state.infra.splash_screen_view_model = Some(
            crate::view_models::splash_screen::SplashScreenViewModel::from_state(
                &state.infra.bootstrap_state,
                &state.repos.recent_repos,
                state.repos.selected_repo,
                state.ui.spinner_frame,
                BAR_WIDTH,
                &state.theme,
            ),
        );
    } else {
        // Clear view model when splash screen should not be shown
        state.infra.splash_screen_view_model = None;
    }
}

/// Recompute shortcuts panel view model when panel is shown or scrolled
fn recompute_shortcuts_panel_view_model(state: &mut UiState, theme: &crate::theme::Theme) {
    if state.show_shortcuts {
        // Use reasonable default for typical terminal dimensions
        // Typical terminal: 80x24, popup: 80%, inner after margins: ~18 rows
        const DEFAULT_VISIBLE_HEIGHT: usize = 18;

        state.shortcuts_panel_view_model = Some(
            crate::view_models::shortcuts_panel::ShortcutsPanelViewModel::from_state(
                crate::shortcuts::get_shortcuts(),
                state.shortcuts_scroll,
                DEFAULT_VISIBLE_HEIGHT,
                theme,
            ),
        );

        // Update max scroll in state (returned from view model)
        if let Some(ref vm) = state.shortcuts_panel_view_model {
            state.shortcuts_max_scroll = vm.max_scroll;
        }
    }
}

/// Parse GitHub URL into (org, repo, branch)
/// Supports formats:
/// - `https://github.com/org/repo`
/// - `https://github.com/org/repo.git`
/// - `https://github.com/org/repo/tree/branch`
/// - `github.com/org/repo`
fn parse_github_url(url: &str) -> Option<(String, String, String)> {
    let url = url.trim();

    // Remove protocol if present
    let url = url.strip_prefix("https://").unwrap_or(url);
    let url = url.strip_prefix("http://").unwrap_or(url);

    // Remove github.com prefix
    let url = url
        .strip_prefix("github.com/")
        .or_else(|| url.strip_prefix("www.github.com/"))?;

    // Split by '/'
    let parts: Vec<&str> = url.split('/').collect();

    if parts.len() >= 2 {
        let org = parts[0].to_string();
        // Remove .git suffix if present
        let mut repo = parts[1].to_string();
        if repo.ends_with(".git") {
            repo = repo.strip_suffix(".git").unwrap().to_string();
        }

        let branch = if parts.len() >= 4 && parts[2] == "tree" {
            parts[3].to_string()
        } else {
            "main".to_string()
        };

        Some((org, repo, branch))
    } else {
        None
    }
}

/// Infrastructure reducer - manages GitHub client and bootstrap process
/// Handles initialization of external services
fn infrastructure_reducer(
    mut state: InfrastructureState,
    action: &Action,
) -> (InfrastructureState, Vec<Effect>) {
    match action {
        Action::Bootstrap => {
            // Start bootstrap sequence
            state.bootstrap_state = BootstrapState::LoadingRepositories;

            // MIGRATION NOTE: Bootstrap flow now handled by TaskMiddleware
            // Middleware will dispatch OctocrabInitialized → BootstrapComplete
            // Old: vec![]
            (state, vec![])
        }
        Action::OctocrabInitialized(client) => {
            // Store initialized Octocrab client in state (reducer responsibility)
            state.octocrab = Some(client.clone());

            // MIGRATION NOTE: Repo loading now handled by TaskMiddleware
            // Middleware will dispatch BootstrapComplete after loading repos
            // Old: vec![]
            (state, vec![])
        }
        Action::SetBootstrapState(new_state) => {
            state.bootstrap_state = new_state.clone();
            (state, vec![])
        }
        Action::BootstrapComplete(result) => {
            // Start recurring updates when bootstrap completes
            let mut effects = vec![];
            match result {
                Ok(_) => {
                    state.bootstrap_state = BootstrapState::UIReady;
                    // Start recurring updates every 30 minutes (1800000 milliseconds)
                    const THIRTY_MINUTES_MS: u64 = 30 * 60 * 1000;
                    debug!("Bootstrap completed, starting recurring updates");
                }
                Err(_) => {
                    state.bootstrap_state =
                        BootstrapState::Error(result.as_ref().unwrap_err().clone());
                }
            }
            (state, effects)
        }
        _ => (state, vec![]),
    }
}

/// Repository and PR state reducer
/// ALL logic lives here - reducer returns effects to be performed
fn repos_reducer(
    mut state: ReposState,
    action: &Action,
    config: &crate::config::Config,
    theme: &crate::theme::Theme,
    infrastructure: &InfrastructureState,
) -> (ReposState, Vec<Effect>) {
    let mut effects = vec![];

    match action {
        // Bootstrap: Load repositories and session
        Action::Bootstrap => {
            info!("Starting application bootstrap (repos_reducer)...");
            // Note: infrastructure_reducer handles LoadEnvFile, InitializeOctocrab, LoadRepositories
            // repos_reducer just acknowledges bootstrap started
        }

        // Internal state update actions (bootstrap_state moved to infrastructure_reducer)
        Action::SetBootstrapState(_) => {
            // Handled by infrastructure_reducer
        }
        Action::SetLoadingState(new_state) => {
            state.loading_state = new_state.clone();
        }
        Action::SetReposLoading(indices) => {
            for &index in indices {
                let data = state.repo_data.entry(index).or_default();
                data.loading_state = LoadingState::Loading;
            }
        }

        // Repositories loaded - restore session and load PRs
        Action::BootstrapComplete(Ok(result)) => {
            info!(
                "Bootstrap complete: {} repositories configured, selected: {}",
                result.repos.len(),
                result.selected_repo
            );
            state.recent_repos = result.repos.clone();
            state.selected_repo = result.selected_repo;

            // Load selected repo first for quick UI display
            if let Some(selected_repo) = result.repos.get(result.selected_repo) {
                info!(
                    "Loading selected repo first: {}/{}",
                    selected_repo.org, selected_repo.repo
                );
                let data = state.repo_data.entry(result.selected_repo).or_default();
                data.loading_state = LoadingState::Loading;

                // Effect: Load just the selected repo first (use cache for fast startup)

                // Effect: Show status message
            } else {
                // Fallback: load all repos if selected repo doesn't exist

                for i in 0..result.repos.len() {
                    let data = state.repo_data.entry(i).or_default();
                    data.loading_state = LoadingState::Loading;
                }

                // Collect all repos with their indices
                let repos_with_indices: Vec<_> = result
                    .repos
                    .iter()
                    .enumerate()
                    .map(|(i, repo)| (i, repo.clone()))
                    .collect();
            }
        }
        Action::BootstrapComplete(Err(err)) => {}
        Action::RepoLoadingStarted(repo_index) => {
            // Mark repo as loading (request in flight)
            let data = state.repo_data.entry(*repo_index).or_default();
            data.loading_state = LoadingState::Loading;
        }
        Action::DeleteCurrentRepo => {
            // Delete the currently selected repository
            if !state.recent_repos.is_empty() {
                let selected_idx = state.selected_repo;

                // Remove the repo from the list
                state.recent_repos.remove(selected_idx);

                // Remove its data
                state.repo_data.remove(&selected_idx);

                // Rebuild repo_data with updated indices
                let mut new_repo_data = std::collections::HashMap::new();
                for (old_idx, data) in state.repo_data.iter() {
                    let new_idx = if *old_idx > selected_idx {
                        old_idx - 1
                    } else {
                        *old_idx
                    };
                    new_repo_data.insert(new_idx, data.clone());
                }
                state.repo_data = new_repo_data;

                // Adjust selected repo index
                if state.recent_repos.is_empty() {
                    state.selected_repo = 0;
                    state.prs.clear();
                    state.loading_state = LoadingState::Idle;
                    state.state.select(None);
                } else if selected_idx >= state.recent_repos.len() {
                    // Was last repo, select the new last one
                    state.selected_repo = state.recent_repos.len() - 1;
                    // Sync legacy fields with new selection
                    if let Some(data) = state.repo_data.get(&state.selected_repo) {
                        state.prs = data.prs.clone();
                        state.state = data.table_state.clone();
                        state.loading_state = data.loading_state.clone();
                    }
                } else {
                    // Sync legacy fields with current selection
                    if let Some(data) = state.repo_data.get(&state.selected_repo) {
                        state.prs = data.prs.clone();
                        state.state = data.table_state.clone();
                        state.loading_state = data.loading_state.clone();
                    }
                }

                // MIGRATION NOTE: SaveRepositories now handled by TaskMiddleware
                // Middleware will:
                // - Save to file
                // - Dispatch SetTaskStatus with success/error message
            }
        }
        Action::RepositoryAdded { repo_index, repo } => {
            // Add repository to state (dispatched from effect after file save)
            state.recent_repos.push(repo.clone());

            // Initialize repo data
            let data = state.repo_data.entry(*repo_index).or_default();
            data.loading_state = LoadingState::Loading;

            // Recompute view model if this becomes the selected repo
            if *repo_index == state.selected_repo {
                recompute_pr_table_view_model(&mut state, theme);
            }
        }
        Action::SelectRepoByIndex(index) => {
            if *index < state.recent_repos.len() {
                state.selected_repo = *index;

                // Sync legacy fields with repo_data
                if let Some(data) = state.repo_data.get(index) {
                    state.prs = data.prs.clone();
                    state.state = data.table_state.clone();
                    state.loading_state = data.loading_state.clone();
                }

                // Recompute view model for new repo
                recompute_pr_table_view_model(&mut state, theme);
            }
        }
        Action::RepoDataLoaded(repo_index, Ok(prs)) => {
            let data = state.repo_data.entry(*repo_index).or_default();
            data.prs = prs.clone();
            data.loading_state = LoadingState::Loaded;
            data.last_updated = Some(chrono::Local::now());

            // Update table selection based on PR list
            if data.prs.is_empty() {
                // Clear selection and selected PRs when no PRs
                data.table_state.select(None);
                data.selected_pr_numbers.clear();
            } else if data.table_state.selected().is_none() {
                // Select first row if nothing selected
                data.table_state.select(Some(0));
            }

            // Validate selected_pr_numbers - remove PRs that no longer exist
            // This is critical after filtering or when PRs are closed/merged
            let current_pr_numbers: std::collections::HashSet<_> =
                data.prs.iter().map(PrNumber::from_pr).collect();
            data.selected_pr_numbers
                .retain(|num| current_pr_numbers.contains(num));

            // Sync legacy fields if this is the selected repo
            if *repo_index == state.selected_repo {
                state.prs = prs.clone();
                state.state = data.table_state.clone();
                state.loading_state = LoadingState::Loaded;
            }

            // Effect: Check merge status for loaded PRs
            if let Some(repo) = state.recent_repos.get(*repo_index).cloned() {
                let pr_numbers: Vec<usize> = prs.iter().map(|pr| pr.number).collect();

                // Effect: Check comment counts for loaded PRs
            }

            // Quick load: Check if this is the first repo loaded (selected repo)
            if infrastructure.bootstrap_state == BootstrapState::LoadingFirstRepo
                && *repo_index == state.selected_repo
            {
                // First repo loaded - UI is ready to display!

                // Show success message for first repo
                if let Some(repo) = state.recent_repos.get(*repo_index) {
                    info!(
                        "First repo loaded: {}/{} ({} PRs)",
                        repo.org,
                        repo.repo,
                        prs.len()
                    );
                }

                // Start loading remaining repos in background
                if state.recent_repos.len() > 1 {
                    info!(
                        "Starting background loading of {} remaining repositories",
                        state.recent_repos.len() - 1
                    );

                    // Mark all other repos as loading
                    for i in 0..state.recent_repos.len() {
                        if i != state.selected_repo {
                            let data = state.repo_data.entry(i).or_default();
                            data.loading_state = LoadingState::Loading;
                        }
                    }

                    // Collect repos to load with their indices (all except selected one)
                    let repos_to_load: Vec<_> = state
                        .recent_repos
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != state.selected_repo)
                        .map(|(i, repo)| (i, repo.clone()))
                        .collect();

                    // Effect: Load remaining repos
                } else {
                    // Only one repo - we're done
                }
            } else if infrastructure.bootstrap_state == BootstrapState::LoadingRemainingRepos {
                // Show status message for each repo that loads in background
                if let Some(repo) = state.recent_repos.get(*repo_index) {
                    info!(
                        "Background repo loaded: {}/{} ({} PRs)",
                        repo.org,
                        repo.repo,
                        prs.len()
                    );
                }
            }

            // Check if all repos are done loading
            let all_loaded = state.repo_data.len() == state.recent_repos.len()
                && state.repo_data.values().all(|d| {
                    matches!(
                        d.loading_state,
                        LoadingState::Loaded | LoadingState::Error(_)
                    )
                });

            // Effect: Dispatch bootstrap completion
            if all_loaded && infrastructure.bootstrap_state == BootstrapState::LoadingRemainingRepos
            {
                let loaded_count = state
                    .repo_data
                    .values()
                    .filter(|d| matches!(d.loading_state, LoadingState::Loaded))
                    .count();
                info!(
                    "All repositories loaded: {}/{} successful",
                    loaded_count,
                    state.recent_repos.len()
                );
            }

            // Recompute view model after PR data loaded
            if *repo_index == state.selected_repo {
                recompute_pr_table_view_model(&mut state, theme);
                recompute_repository_tabs_view_model(&mut state);
            }
        }
        Action::RepoDataLoaded(repo_index, Err(err)) => {
            let data = state.repo_data.entry(*repo_index).or_default();
            data.loading_state = LoadingState::Error(err.clone());

            // Quick load: Check if this is the first repo that failed
            if infrastructure.bootstrap_state == BootstrapState::LoadingFirstRepo
                && *repo_index == state.selected_repo
            {
                // First repo failed - still show UI but with error message

                // Show error message for first repo
                if let Some(repo) = state.recent_repos.get(*repo_index) {
                    error!(
                        "Failed to load first repo {}/{}: {}",
                        repo.org, repo.repo, err
                    );
                }

                // Start loading remaining repos in background (if any)
                if state.recent_repos.len() > 1 {
                    // Mark all other repos as loading
                    for i in 0..state.recent_repos.len() {
                        if i != state.selected_repo {
                            let data = state.repo_data.entry(i).or_default();
                            data.loading_state = LoadingState::Loading;
                        }
                    }

                    // Collect repos to load with their indices (all except selected one)
                    let repos_to_load: Vec<_> = state
                        .recent_repos
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != state.selected_repo)
                        .map(|(i, repo)| (i, repo.clone()))
                        .collect();

                    // Effect: Load remaining repos
                } else {
                    // Only one repo and it failed - still complete bootstrap to show UI
                }
            } else if infrastructure.bootstrap_state == BootstrapState::LoadingRemainingRepos {
                // Show error message for background repo that failed
                if let Some(repo) = state.recent_repos.get(*repo_index) {
                    error!(
                        "Failed to load background repo {}/{}: {}",
                        repo.org, repo.repo, err
                    );
                }
            }

            // Check if all repos are done loading (even with errors)
            let all_loaded = state.repo_data.len() == state.recent_repos.len()
                && state.repo_data.values().all(|d| {
                    matches!(
                        d.loading_state,
                        LoadingState::Loaded | LoadingState::Error(_)
                    )
                });

            // Effect: Dispatch bootstrap completion if all done
            if all_loaded && infrastructure.bootstrap_state == BootstrapState::LoadingRemainingRepos
            {
                let loaded_count = state
                    .repo_data
                    .values()
                    .filter(|d| matches!(d.loading_state, LoadingState::Loaded))
                    .count();
                let error_count = state
                    .repo_data
                    .values()
                    .filter(|d| matches!(d.loading_state, LoadingState::Error(_)))
                    .count();
            }
        }
        Action::CycleFilter => {
            state.filter = state.filter.next();

            // Reload current repository with new filter (use cache, filter is client-side)
            if let Some(repo) = state.recent_repos.get(state.selected_repo).cloned() {}

            // Note: View model will be recomputed when RepoDataLoaded action fires
        }
        Action::NavigateToNextPr => {
            let i = match state.state.selected() {
                Some(i) => {
                    if i >= state.prs.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            state.state.select(Some(i));

            // Sync to repo_data
            if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.table_state.select(Some(i));
            }

            // Recompute view model (cursor position changed)
            recompute_pr_table_view_model(&mut state, theme);
        }
        Action::NavigateToPreviousPr => {
            let i = match state.state.selected() {
                Some(i) => {
                    if i == 0 {
                        state.prs.len().saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            state.state.select(Some(i));

            // Sync to repo_data
            if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.table_state.select(Some(i));
            }

            // Recompute view model (cursor position changed)
            recompute_pr_table_view_model(&mut state, theme);
        }
        Action::TogglePrSelection => {
            if let Some(selected) = state.state.selected()
                && selected < state.prs.len()
            {
                let pr_number = PrNumber::from_pr(&state.prs[selected]);

                // Update type-safe PR number-based selection (stable across filtering)
                if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                    if data.selected_pr_numbers.contains(&pr_number) {
                        data.selected_pr_numbers.remove(&pr_number);
                    } else {
                        data.selected_pr_numbers.insert(pr_number);
                    }
                }

                // Recompute view model (selection changed)
                recompute_pr_table_view_model(&mut state, theme);

                // Automatically advance to next PR if not on the last row
                // Note: NavigateToNextPr will trigger another recompute, but that's OK
                if selected < state.prs.len().saturating_sub(1) {}
            }
        }
        Action::ClearPrSelection => {
            // Clear all PR selections for the current repo
            if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.selected_pr_numbers.clear();
            }

            // Recompute view model (selection changed)
            recompute_pr_table_view_model(&mut state, theme);
        }
        Action::SelectAllPrs => {
            // Select all PRs for the current repo
            if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.selected_pr_numbers = data
                    .prs
                    .iter()
                    .map(crate::state::PrNumber::from_pr)
                    .collect();
            }

            // Recompute view model (selection changed)
            recompute_pr_table_view_model(&mut state, theme);
        }
        Action::DeselectAllPrs => {
            // Deselect all PRs for the current repo (same as ClearPrSelection)
            if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.selected_pr_numbers.clear();
            }

            // Recompute view model (selection changed)
            recompute_pr_table_view_model(&mut state, theme);
        }
        Action::MergeStatusUpdated(repo_index, pr_number, status) => {
            // Update PR status in repo_data
            if let Some(data) = state.repo_data.get_mut(repo_index)
                && let Some(pr) = data.prs.iter_mut().find(|p| p.number == *pr_number)
            {
                pr.mergeable = *status;
            }

            // Sync legacy fields if this is the selected repo
            if *repo_index == state.selected_repo
                && let Some(pr) = state.prs.iter_mut().find(|p| p.number == *pr_number)
            {
                pr.mergeable = *status;
            }

            // Recompute view model if this is the selected repo (status changed)
            if *repo_index == state.selected_repo {
                recompute_pr_table_view_model(&mut state, theme);
            }

            // If status is BuildInProgress, start monitoring the build
            if *status == crate::pr::MergeableStatus::BuildInProgress
                && let Some(repo) = state.recent_repos.get(*repo_index).cloned()
            {
                // First dispatch action to update state immediately

                // Then start background monitoring
            }
        }
        Action::RebaseStatusUpdated(repo_index, pr_number, needs_rebase) => {
            // Update PR rebase status in repo_data
            if let Some(data) = state.repo_data.get_mut(repo_index)
                && let Some(pr) = data.prs.iter_mut().find(|p| p.number == *pr_number)
            {
                pr.needs_rebase = *needs_rebase;
            }

            // Sync legacy fields if this is the selected repo
            if *repo_index == state.selected_repo
                && let Some(pr) = state.prs.iter_mut().find(|p| p.number == *pr_number)
            {
                pr.needs_rebase = *needs_rebase;
            }
        }
        Action::CommentCountUpdated(repo_index, pr_number, comment_count) => {
            // Update PR comment count in repo_data
            if let Some(data) = state.repo_data.get_mut(repo_index)
                && let Some(pr) = data.prs.iter_mut().find(|p| p.number == *pr_number)
            {
                pr.no_comments = *comment_count;
            }

            // Sync legacy fields if this is the selected repo
            if *repo_index == state.selected_repo
                && let Some(pr) = state.prs.iter_mut().find(|p| p.number == *pr_number)
            {
                pr.no_comments = *comment_count;
            }
        }
        Action::MergeComplete(Ok(_)) => {
            // Clear selections after successful merge (only if not in merge bot)
            if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.selected_pr_numbers.clear();
            }
        }
        Action::ClosePrComplete(Ok(_)) => {
            // Clear selections after successful close
            if let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.selected_pr_numbers.clear();
            }

            // Schedule delayed reload (500ms) to give GitHub time to process the close
            info!(
                "Scheduling delayed reload after closing PR(s) for repo #{}",
                state.selected_repo
            );
        }
        Action::RefreshCurrentRepo => {
            // MIGRATION NOTE: Reload now handled by TaskMiddleware
            // Middleware dispatches SetReposLoading and SetTaskStatus
            // Old: Effect::LoadSingleRepo with bypass_cache: true
            // No effects needed - middleware handles everything
        }
        Action::ReloadRepo(_repo_index) => {
            // MIGRATION NOTE: Reload now handled by TaskMiddleware
            // Middleware dispatches SetReposLoading
            // Old: Effect::LoadSingleRepo with bypass_cache: true
            // No effects needed - middleware handles everything
        }
        Action::StartRecurringUpdates(interval_ms) => {
            // Effect: Start background recurring task to update all repos periodically
            debug!(
                "Starting recurring updates with interval: {}ms ({} minutes)",
                interval_ms,
                interval_ms / 60000
            );
        }
        Action::RecurringUpdateTriggered => {
            // Effect: Reload all repositories (triggered by recurring background task)
            debug!("Recurring update triggered, reloading all repos");
            for (repo_index, repo) in state.recent_repos.iter().enumerate() {}
        }
        Action::Rebase => {
            // MIGRATION NOTE: PerformRebase and StartOperationMonitoring now handled by TaskMiddleware
            // Middleware will:
            // - Get selected PRs from state
            // - Dispatch StartOperationMonitor for each PR
            // - Dispatch SetTaskStatus
            // - Send BackgroundTask::Rebase
            // Just clear selection here
            let has_selection = if let Some(data) = state.repo_data.get(&state.selected_repo) {
                !data.selected_pr_numbers.is_empty()
            } else {
                false
            };

            if has_selection && let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.selected_pr_numbers.clear();
            }
        }
        Action::RerunFailedJobs => {
            // Effect: Rerun failed CI jobs for current or selected PRs
            if let Some(repo) = state.recent_repos.get(state.selected_repo).cloned() {
                // Use PR numbers for stable selection
                let has_selection = if let Some(data) = state.repo_data.get(&state.selected_repo) {
                    !data.selected_pr_numbers.is_empty()
                } else {
                    false
                };

                let pr_numbers: Vec<usize> = if !has_selection {
                    // Rerun for current PR only
                    state
                        .state
                        .selected()
                        .and_then(|idx| state.prs.get(idx))
                        .map(|pr| vec![pr.number])
                        .unwrap_or_default()
                } else if let Some(data) = state.repo_data.get(&state.selected_repo) {
                    // Rerun for selected PRs using PR numbers (stable)
                    state
                        .prs
                        .iter()
                        .filter(|pr| data.selected_pr_numbers.contains(&PrNumber::from_pr(pr)))
                        .map(|pr| pr.number)
                        .collect()
                } else {
                    Vec::new()
                };

                if !pr_numbers.is_empty() {}
            }
        }
        Action::ApprovePrs => {
            // MIGRATION NOTE: ApprovePrs now handled by TaskMiddleware
            // Middleware will:
            // - Get selected PRs from state
            // - Dispatch SetTaskStatus
            // - Send BackgroundTask::ApprovePrs
            // No effects needed
        }
        Action::MergeSelectedPrs => {
            // MIGRATION NOTE: PerformMerge, StartOperationMonitoring, and EnableAutoMerge now handled by TaskMiddleware
            // Middleware will:
            // - Get selected PRs from state
            // - Separate PRs by status (ready vs building)
            // - Dispatch StartOperationMonitor for each PR being merged
            // - Dispatch SetTaskStatus
            // - Send BackgroundTask::Merge for ready PRs
            // - Send BackgroundTask::EnableAutoMerge for building PRs
            // Just clear selection here
            let has_selection = if let Some(data) = state.repo_data.get(&state.selected_repo) {
                !data.selected_pr_numbers.is_empty()
            } else {
                false
            };

            if has_selection && let Some(data) = state.repo_data.get_mut(&state.selected_repo) {
                data.selected_pr_numbers.clear();
            }
        }
        Action::StartMergeBot => {
            // Effect: Start merge bot with selected PRs
            if let Some(repo) = state.recent_repos.get(state.selected_repo).cloned() {
                // Use PR numbers for stable selection
                let prs_to_process: Vec<_> =
                    if let Some(data) = state.repo_data.get(&state.selected_repo) {
                        state
                            .prs
                            .iter()
                            .filter(|pr| data.selected_pr_numbers.contains(&PrNumber::from_pr(pr)))
                            .cloned()
                            .collect()
                    } else {
                        Vec::new()
                    };

                if !prs_to_process.is_empty() {}
            }
        }
        Action::OpenCurrentPrInBrowser => {
            // MIGRATION NOTE: OpenInBrowser now handled by TaskMiddleware
            // Middleware will:
            // - Get selected PRs from state
            // - Open each PR URL in browser using platform-specific command
            // No effects needed
        }
        Action::OpenBuildLogs => {
            // Effect: Load build logs for current PR
            if let Some(selected_idx) = state.state.selected()
                && let Some(pr) = state.prs.get(selected_idx).cloned()
                && let Some(repo) = state.recent_repos.get(state.selected_repo).cloned()
            {
            }
        }
        Action::OpenInIDE => {
            // MIGRATION NOTE: OpenInIDE now handled by TaskMiddleware
            // Middleware will:
            // - Get current repo and selected PR
            // - Dispatch SetTaskStatus with progress message
            // - Send BackgroundTask::OpenPRInIDE
            // No effects needed
        }
        Action::SelectNextRepo => {
            if !state.recent_repos.is_empty() {
                state.selected_repo = (state.selected_repo + 1) % state.recent_repos.len();

                // Sync legacy fields with repo_data
                if let Some(data) = state.repo_data.get(&state.selected_repo) {
                    state.prs = data.prs.clone();
                    state.state = data.table_state.clone();
                    state.loading_state = data.loading_state.clone();
                }

                // Recompute view model for new repo
                recompute_pr_table_view_model(&mut state, theme);
                recompute_repository_tabs_view_model(&mut state);
            }
        }
        Action::SelectPreviousRepo => {
            if !state.recent_repos.is_empty() {
                state.selected_repo = if state.selected_repo == 0 {
                    state.recent_repos.len() - 1
                } else {
                    state.selected_repo - 1
                };

                // Sync legacy fields with repo_data
                if let Some(data) = state.repo_data.get(&state.selected_repo) {
                    state.prs = data.prs.clone();
                    state.state = data.table_state.clone();
                    state.loading_state = data.loading_state.clone();
                }

                // Recompute view model for new repo
                recompute_pr_table_view_model(&mut state, theme);
                recompute_repository_tabs_view_model(&mut state);
            }
        }
        Action::StartOperationMonitor(repo_index, pr_number, operation) => {
            // Add PR to operation monitor queue and set initial state
            if let Some(data) = state.repo_data.get_mut(repo_index) {
                // Check if already in queue
                if !data
                    .operation_monitor_queue
                    .iter()
                    .any(|op| op.pr_number == *pr_number)
                {
                    // Set the PR status to Rebasing or Merging immediately
                    let status = match operation {
                        crate::state::OperationType::Rebase => crate::pr::MergeableStatus::Rebasing,
                        crate::state::OperationType::Merge => crate::pr::MergeableStatus::Merging,
                    };

                    // Update PR status in repo_data
                    if let Some(pr) = data.prs.iter_mut().find(|p| p.number == *pr_number) {
                        pr.mergeable = status;
                    }

                    // Also sync to legacy fields if this is the selected repo
                    if *repo_index == state.selected_repo
                        && let Some(pr) = state.prs.iter_mut().find(|p| p.number == *pr_number)
                    {
                        pr.mergeable = status;
                    }

                    // Add to monitoring queue
                    data.operation_monitor_queue
                        .push(crate::state::OperationMonitor {
                            pr_number: *pr_number,
                            operation: *operation,
                            started_at: std::time::Instant::now(),
                            check_count: 0,
                            last_head_sha: None,
                        });
                }
            }
        }
        Action::RemoveFromOperationMonitor(repo_index, pr_number) => {
            // Remove PR from operation monitor queue
            if let Some(data) = state.repo_data.get_mut(repo_index) {
                data.operation_monitor_queue
                    .retain(|op| op.pr_number != *pr_number);
            }
        }
        Action::OperationMonitorCheck(repo_index, pr_number) => {
            // Periodic status check for operation monitor
            // This will be handled by the background task which will dispatch
            // MergeStatusUpdated actions based on GitHub API responses
            // For now, just increment check count
            if let Some(data) = state.repo_data.get_mut(repo_index)
                && let Some(monitor) = data
                    .operation_monitor_queue
                    .iter_mut()
                    .find(|op| op.pr_number == *pr_number)
            {
                monitor.check_count += 1;

                // Timeout after 120 checks (1 hour at 30s intervals)
                if monitor.check_count >= 120 {
                    // Remove from queue - timeout reached
                    data.operation_monitor_queue
                        .retain(|op| op.pr_number != *pr_number);
                }
            }
        }
        Action::AddToAutoMergeQueue(repo_index, pr_number) => {
            // Add PR to auto-merge queue
            if let Some(data) = state.repo_data.get_mut(repo_index) {
                // Check if already in queue
                if !data
                    .auto_merge_queue
                    .iter()
                    .any(|pr| pr.pr_number == *pr_number)
                {
                    data.auto_merge_queue.push(crate::state::AutoMergePR {
                        pr_number: *pr_number,
                        started_at: std::time::Instant::now(),
                        check_count: 0,
                    });
                }
            }
        }
        Action::RemoveFromAutoMergeQueue(repo_index, pr_number) => {
            // Remove PR from auto-merge queue
            if let Some(data) = state.repo_data.get_mut(repo_index) {
                data.auto_merge_queue
                    .retain(|pr| pr.pr_number != *pr_number);
            }
        }
        Action::AutoMergeStatusCheck(repo_index, pr_number) => {
            // Periodic status check for auto-merge PR
            if let Some(data) = state.repo_data.get_mut(repo_index)
                && let Some(auto_pr) = data
                    .auto_merge_queue
                    .iter_mut()
                    .find(|pr| pr.pr_number == *pr_number)
            {
                auto_pr.check_count += 1;

                // Check if we've exceeded the time limit (20 minutes = 20 checks at 1 min intervals)
                if auto_pr.check_count >= 20 {
                    // Remove from queue - timeout reached
                    data.auto_merge_queue
                        .retain(|pr| pr.pr_number != *pr_number);
                } else {
                    // Check PR status
                    if let Some(repo) = state.recent_repos.get(*repo_index).cloned() {
                        // Find the PR to check its status
                        if let Some(pr) = data.prs.iter().find(|p| p.number == *pr_number) {
                            match pr.mergeable {
                                crate::pr::MergeableStatus::Ready => {
                                    // PR is ready - trigger merge

                                    // Remove from queue
                                    data.auto_merge_queue.retain(|p| p.pr_number != *pr_number);
                                }
                                crate::pr::MergeableStatus::BuildFailed => {
                                    // Build failed - stop monitoring
                                    data.auto_merge_queue.retain(|p| p.pr_number != *pr_number);
                                }
                                crate::pr::MergeableStatus::NeedsRebase => {
                                    // Needs rebase - stop monitoring
                                    data.auto_merge_queue.retain(|p| p.pr_number != *pr_number);
                                }
                                crate::pr::MergeableStatus::BuildInProgress => {
                                    // Still building - schedule next check
                                    // This will be handled by the background task
                                }
                                _ => {
                                    // Unknown or conflicted - continue monitoring
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    (state, effects)
}

/// Log panel state reducer
fn log_panel_reducer(
    mut state: LogPanelState,
    action: &Action,
    theme: &crate::theme::Theme,
) -> (LogPanelState, Vec<Effect>) {
    match action {
        Action::BuildLogsLoaded(jobs, pr_context) => {
            // Create master-detail log panel from job logs
            state.panel = Some(crate::log::create_log_panel_from_jobs(
                jobs.clone(),
                pr_context.clone(),
            ));
            // Recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::CloseLogPanel => {
            state.panel = None;
            state.view_model = None;
        }
        Action::ScrollLogPanelUp => {
            if let Some(ref mut panel) = state.panel {
                panel.scroll_offset = panel.scroll_offset.saturating_sub(1);
            }
        }
        Action::ScrollLogPanelDown => {
            if let Some(ref mut panel) = state.panel {
                panel.scroll_offset = panel.scroll_offset.saturating_add(1);
            }
        }
        Action::PageLogPanelDown => {
            if let Some(ref mut panel) = state.panel {
                // Page down by viewport_height - 1 (keep one line of context)
                let page_size = panel.viewport_height.saturating_sub(1).max(1);
                panel.scroll_offset = panel.scroll_offset.saturating_add(page_size);
            }
        }
        Action::ScrollLogPanelLeft => {
            if let Some(ref mut panel) = state.panel {
                // Scroll left by 5 characters for better UX
                panel.horizontal_scroll = panel.horizontal_scroll.saturating_sub(5);
            }
            // Horizontal scroll affects display text - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::ScrollLogPanelRight => {
            if let Some(ref mut panel) = state.panel {
                // Scroll right by 5 characters for better UX
                panel.horizontal_scroll = panel.horizontal_scroll.saturating_add(5);
            }
            // Horizontal scroll affects display text - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::NextLogSection => {
            if let Some(ref mut panel) = state.panel {
                panel.find_next_error();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::PrevLogSection => {
            if let Some(ref mut panel) = state.panel {
                // Find previous error - move up until we find a node with errors
                // For now, just navigate up
                panel.navigate_up();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::ToggleTimestamps => {
            if let Some(ref mut panel) = state.panel {
                panel.show_timestamps = !panel.show_timestamps;
            }
            // Timestamp display changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::UpdateLogPanelViewport(height) => {
            if let Some(ref mut panel) = state.panel {
                panel.viewport_height = *height;
            }
        }
        // Tree navigation
        Action::SelectNextJob => {
            if let Some(ref mut panel) = state.panel {
                panel.navigate_down();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::SelectPrevJob => {
            if let Some(ref mut panel) = state.panel {
                panel.navigate_up();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::ToggleTreeNode => {
            if let Some(ref mut panel) = state.panel {
                panel.toggle_at_cursor();
            }
            // Tree expansion changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::FocusJobList => {
            // No-op in tree view - unified view has no separate focus
        }
        Action::FocusLogViewer => {
            // No-op in tree view - unified view has no separate focus
        }
        // Step navigation
        Action::NextStep => {
            if let Some(ref mut panel) = state.panel {
                panel.navigate_down();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::PrevStep => {
            if let Some(ref mut panel) = state.panel {
                panel.navigate_up();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        // Error navigation
        Action::NextError => {
            if let Some(ref mut panel) = state.panel {
                panel.find_next_error();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        Action::PrevError => {
            if let Some(ref mut panel) = state.panel {
                panel.find_prev_error();
            }
            // Cursor position changed - recompute view model
            recompute_view_model(&mut state, theme);
        }
        _ => {}
    }

    (state, vec![])
}

/// Helper function to recompute view model from panel
fn recompute_view_model(state: &mut LogPanelState, theme: &crate::theme::Theme) {
    if let Some(ref panel) = state.panel {
        state.view_model =
            Some(crate::view_models::log_panel::LogPanelViewModel::from_log_panel(panel, theme));
    } else {
        state.view_model = None;
    }
}

/// Helper function to recompute PR table view model
fn recompute_pr_table_view_model(state: &mut ReposState, theme: &crate::theme::Theme) {
    if let Some(selected_repo) = state.recent_repos.get(state.selected_repo) {
        let repo_data = state.repo_data.entry(state.selected_repo).or_default();
        let cursor_index = repo_data.table_state.selected();

        repo_data.pr_table_view_model = Some(
            crate::view_models::pr_table::PrTableViewModel::from_repo_data(
                repo_data,
                selected_repo,
                cursor_index,
                theme,
            ),
        );
    }
}

/// Recompute repository tabs view model after state changes
fn recompute_repository_tabs_view_model(state: &mut ReposState) {
    state.repository_tabs_view_model = Some(
        crate::view_models::repository_tabs::RepositoryTabsViewModel::from_state(
            &state.recent_repos,
            &state.repo_data,
            state.selected_repo,
            state.filter.label(),
        ),
    );
}

/// Merge bot state reducer
fn merge_bot_reducer(
    mut state: MergeBotState,
    action: &Action,
    repos: &ReposState,
) -> (MergeBotState, Vec<Effect>) {
    let mut effects = vec![];

    match action {
        Action::StartMergeBot => {
            // Note: actual bot starting logic with PR data happens in the effect handler
            // This just ensures the state is ready
        }
        Action::StartMergeBotWithPrData(pr_data) => {
            // Initialize merge bot with PR data (reducer responsibility)
            state.bot.start(pr_data.clone());
        }
        Action::MergeBotTick => {
            // Process merge bot queue if bot is running
            if state.bot.is_running()
                && let Some(repo) = repos.recent_repos.get(repos.selected_repo).cloned()
            {
                let repo_data = repos
                    .repo_data
                    .get(&repos.selected_repo)
                    .cloned()
                    .unwrap_or_default();

                // Process next PR in queue
                if let Some(bot_action) = state.bot.process_next(&repo_data.prs) {
                    use crate::merge_bot::MergeBotAction;
                    match bot_action {
                        MergeBotAction::DispatchMerge(_indices) => {}
                        MergeBotAction::DispatchRebase(_indices) => {}
                        MergeBotAction::WaitForCI(_pr_number) => {}
                        MergeBotAction::PollMergeStatus(pr_number, is_checking_ci) => {}
                        MergeBotAction::PrSkipped(_pr_number, _reason) => {}
                        MergeBotAction::Completed => {

                            // Refresh the PR list (bypass cache after merge operations)
                        }
                    }
                }
            }
        }
        Action::MergeStatusUpdated(_repo_index, pr_number, status) => {
            if state.bot.is_running() {
                state.bot.handle_status_update(*pr_number, *status);
            }
        }
        Action::RebaseComplete(result) => {
            if state.bot.is_running() {
                state.bot.handle_rebase_complete(result.is_ok());
            }
        }
        Action::MergeComplete(result) => {
            if state.bot.is_running() {
                state.bot.handle_merge_complete(result.is_ok());
            }
        }
        Action::PRMergedConfirmed(_repo_index, pr_number, is_merged) => {
            if state.bot.is_running() {
                state.bot.handle_pr_merged_confirmed(*pr_number, *is_merged);
            }
        }
        _ => {}
    }

    (state, effects)
}

/// Task status reducer
fn task_reducer(mut state: TaskState, action: &Action) -> (TaskState, Vec<Effect>) {
    match action {
        // Internal state update action
        Action::SetTaskStatus(new_status) => {
            state.status = new_status.clone();
        }

        Action::RefreshCurrentRepo => {
            state.status = Some(TaskStatus {
                message: "Refreshing...".to_string(),
                status_type: TaskStatusType::Running,
            });
        }
        Action::RebaseComplete(result) => {
            state.status = Some(match result {
                Ok(_) => TaskStatus {
                    message: "Rebase completed successfully".to_string(),
                    status_type: TaskStatusType::Success,
                },
                Err(err) => TaskStatus {
                    message: format!("Rebase failed: {}", err),
                    status_type: TaskStatusType::Error,
                },
            });
        }
        Action::MergeComplete(result) => {
            state.status = Some(match result {
                Ok(_) => TaskStatus {
                    message: "Merge completed successfully".to_string(),
                    status_type: TaskStatusType::Success,
                },
                Err(err) => TaskStatus {
                    message: format!("Merge failed: {}", err),
                    status_type: TaskStatusType::Error,
                },
            });
        }
        Action::RerunJobsComplete(result) => {
            state.status = Some(match result {
                Ok(_) => TaskStatus {
                    message: "CI jobs rerun successfully".to_string(),
                    status_type: TaskStatusType::Success,
                },
                Err(err) => TaskStatus {
                    message: format!("Failed to rerun CI jobs: {}", err),
                    status_type: TaskStatusType::Error,
                },
            });
        }
        Action::ApprovalComplete(result) => {
            state.status = Some(match result {
                Ok(_) => TaskStatus {
                    message: "PR(s) approved successfully".to_string(),
                    status_type: TaskStatusType::Success,
                },
                Err(err) => TaskStatus {
                    message: format!("Failed to approve PR(s): {}", err),
                    status_type: TaskStatusType::Error,
                },
            });
        }
        Action::ClosePrComplete(result) => {
            state.status = Some(match result {
                Ok(_) => TaskStatus {
                    message: "PR(s) closed successfully".to_string(),
                    status_type: TaskStatusType::Success,
                },
                Err(err) => TaskStatus {
                    message: format!("Failed to close PR(s): {}", err),
                    status_type: TaskStatusType::Error,
                },
            });
        }
        Action::IDEOpenComplete(result) => {
            state.status = Some(match result {
                Ok(_) => TaskStatus {
                    message: "IDE opened successfully".to_string(),
                    status_type: TaskStatusType::Success,
                },
                Err(err) => TaskStatus {
                    message: format!("Failed to open IDE: {}", err),
                    status_type: TaskStatusType::Error,
                },
            });
        }
        _ => {}
    }

    (state, vec![])
}

/// Debug console state reducer - handles debug console actions
fn debug_console_reducer(
    mut state: DebugConsoleState,
    action: &Action,
    theme: &crate::theme::Theme,
) -> (DebugConsoleState, Vec<Effect>) {
    match action {
        Action::ToggleDebugConsole => {
            state.is_open = !state.is_open;
            // Reset scroll when opening
            if state.is_open {
                state.scroll_offset = 0;
            }
            // Recompute view model if console is now open
            if state.is_open {
                recompute_debug_console_view_model(&mut state, theme);
            }
        }
        Action::ScrollDebugConsoleUp => {
            // Immediate viewport scrolling - scroll up by 1 line
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
            // Disable auto-scroll when manually scrolling
            state.auto_scroll = false;
            // Recompute view model
            recompute_debug_console_view_model(&mut state, theme);
        }
        Action::ScrollDebugConsoleDown => {
            // Immediate viewport scrolling - scroll down by 1 line (with bounds check)
            let log_count = state.logs.lock().map(|logs| logs.len()).unwrap_or(0);
            if log_count > 0 {
                state.scroll_offset = state.scroll_offset.saturating_add(1);
            }
            // Disable auto-scroll when manually scrolling
            state.auto_scroll = false;
            // Recompute view model (will clamp scroll_offset to valid range)
            recompute_debug_console_view_model(&mut state, theme);
        }
        Action::PageDebugConsoleDown => {
            // Page down - scroll by 10 lines
            state.scroll_offset = state.scroll_offset.saturating_add(10);
            // Disable auto-scroll when manually scrolling
            state.auto_scroll = false;
            // Recompute view model (will clamp scroll_offset to valid range)
            recompute_debug_console_view_model(&mut state, theme);
        }
        Action::ToggleDebugAutoScroll => {
            state.auto_scroll = !state.auto_scroll;
            // Recompute view model (auto-scroll will jump to end in view model)
            recompute_debug_console_view_model(&mut state, theme);
        }
        Action::ClearDebugLogs => {
            if let Ok(mut logs) = state.logs.lock() {
                logs.clear();
            }
            state.scroll_offset = 0;
            // Recompute view model
            recompute_debug_console_view_model(&mut state, theme);
        }
        Action::UpdateDebugConsoleViewport(_height) => {
            // Just recompute view model in case logs changed
            recompute_debug_console_view_model(&mut state, theme);
        }
        _ => {}
    }

    (state, vec![])
}

/// Recompute debug console view model after state changes
fn recompute_debug_console_view_model(state: &mut DebugConsoleState, theme: &crate::theme::Theme) {
    // Read logs from buffer
    let logs = match state.logs.lock() {
        Ok(log_buffer) => {
            // Convert VecDeque to Vec for view model
            log_buffer.iter().cloned().collect::<Vec<_>>()
        }
        Err(_) => return, // Skip if can't lock
    };

    // Calculate visible height (console height is 50% of screen, minus 2 for borders)
    // Use a reasonable default - will be accurate enough for scrolling
    const CONSOLE_HEIGHT: usize = 15; // Approximate visible lines
    let visible_height = CONSOLE_HEIGHT;

    state.view_model = Some(
        crate::view_models::debug_console::DebugConsoleViewModel::from_state(
            &logs,
            state.scroll_offset,
            state.auto_scroll,
            visible_height,
            theme,
        ),
    );
}
