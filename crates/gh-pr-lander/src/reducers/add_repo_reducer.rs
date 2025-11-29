//! Add Repository Form Reducer
//!
//! Handles state changes for the add repository form.

use crate::actions::Action;
use crate::state::{AddRepoField, AddRepoFormState};

/// Reduce add repository form state based on actions
pub fn reduce(mut state: AddRepoFormState, action: &Action) -> AddRepoFormState {
    match action {
        Action::AddRepoChar(c) => {
            // Add character to the currently focused field
            match state.focused_field {
                AddRepoField::Url => {
                    state.url.push(*c);
                    // Auto-parse URL on each keystroke
                    state.parse_url_and_update();
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

        Action::AddRepoBackspace => {
            // Remove last character from the currently focused field
            match state.focused_field {
                AddRepoField::Url => {
                    state.url.pop();
                    // Re-parse URL after backspace
                    state.parse_url_and_update();
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

        Action::AddRepoClearField => {
            // Clear entire current field (Cmd+Backspace)
            match state.focused_field {
                AddRepoField::Url => {
                    state.url.clear();
                    // Clear org/repo since they were populated from URL
                    state.org.clear();
                    state.repo.clear();
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

        Action::AddRepoNextField => {
            state.focused_field = state.focused_field.next();
        }

        Action::AddRepoPrevField => {
            state.focused_field = state.focused_field.prev();
        }

        // AddRepoConfirm and AddRepoClose: form reset handled in app_reducer,
        // view management handled in add_repository middleware
        _ => {}
    }

    state
}
