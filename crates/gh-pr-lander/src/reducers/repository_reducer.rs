//! Repository Reducer
//!
//! Handles all repository-related state changes including:
//! - Repository list management (add, remove)
//! - Add repository form state

use crate::actions::RepositoryAction;
use crate::state::{AddRepoField, AddRepoFormState, MainViewState};

/// Reduce repository list state
pub fn reduce_repository(mut state: MainViewState, action: &RepositoryAction) -> MainViewState {
    match action {
        RepositoryAction::OpenRepositoryInBrowser => {
            // Side effect handled by middleware
        }
        RepositoryAction::AddRepository(repo) => {
            log::info!("Adding repository: {}", repo.display_name());
            state.repositories.push(repo.clone());
        }
        RepositoryAction::RemoveCurrentRepository => {
            if !state.repositories.is_empty() {
                let idx = state.selected_repository;
                let removed = state.repositories.remove(idx);
                log::info!("Removed repository: {}", removed.display_name());

                // Also remove associated repo_data
                state.repo_data.remove(&idx);

                // Re-index repo_data for indices above the removed one
                let keys_to_update: Vec<usize> =
                    state.repo_data.keys().filter(|&&k| k > idx).copied().collect();
                for old_key in keys_to_update {
                    if let Some(data) = state.repo_data.remove(&old_key) {
                        state.repo_data.insert(old_key - 1, data);
                    }
                }

                // Adjust selected index if necessary
                if state.selected_repository >= state.repositories.len()
                    && !state.repositories.is_empty()
                {
                    state.selected_repository = state.repositories.len() - 1;
                }
            }
        }
        RepositoryAction::LoadRepositoryData(_) => {
            // Side effect handled by middleware
        }
        // Form actions don't affect MainViewState
        RepositoryAction::FormNextField
        | RepositoryAction::FormPrevField
        | RepositoryAction::FormChar(_)
        | RepositoryAction::FormBackspace
        | RepositoryAction::FormClearField
        | RepositoryAction::FormConfirm
        | RepositoryAction::FormClose => {}
    }
    state
}

/// Reduce add repository form state
pub fn reduce_add_repo_form(
    mut state: AddRepoFormState,
    action: &RepositoryAction,
) -> AddRepoFormState {
    match action {
        RepositoryAction::FormChar(c) => {
            match state.focused_field {
                AddRepoField::Url => {
                    state.url.push(*c);
                    state.parse_url_and_update();
                }
                AddRepoField::Host => {
                    state.host.push(*c);
                }
                AddRepoField::Org => {
                    state.org.push(*c);
                }
                AddRepoField::Repo => {
                    state.repo.push(*c);
                }
                AddRepoField::Branch => {
                    state.branch.push(*c);
                }
            }
        }

        RepositoryAction::FormBackspace => {
            match state.focused_field {
                AddRepoField::Url => {
                    state.url.pop();
                    state.parse_url_and_update();
                }
                AddRepoField::Host => {
                    state.host.pop();
                }
                AddRepoField::Org => {
                    state.org.pop();
                }
                AddRepoField::Repo => {
                    state.repo.pop();
                }
                AddRepoField::Branch => {
                    state.branch.pop();
                }
            }
        }

        RepositoryAction::FormClearField => {
            match state.focused_field {
                AddRepoField::Url => {
                    state.url.clear();
                    state.host.clear();
                    state.org.clear();
                    state.repo.clear();
                }
                AddRepoField::Host => {
                    state.host.clear();
                }
                AddRepoField::Org => {
                    state.org.clear();
                }
                AddRepoField::Repo => {
                    state.repo.clear();
                }
                AddRepoField::Branch => {
                    state.branch.clear();
                }
            }
        }

        RepositoryAction::FormNextField => {
            state.focused_field = state.focused_field.next();
        }

        RepositoryAction::FormPrevField => {
            state.focused_field = state.focused_field.prev();
        }

        RepositoryAction::FormConfirm => {
            if state.is_valid() {
                state.reset();
            }
        }

        RepositoryAction::FormClose => {
            state.reset();
        }

        // Non-form actions don't affect form state
        RepositoryAction::OpenRepositoryInBrowser
        | RepositoryAction::AddRepository(_)
        | RepositoryAction::RemoveCurrentRepository
        | RepositoryAction::LoadRepositoryData(_) => {}
    }

    state
}
