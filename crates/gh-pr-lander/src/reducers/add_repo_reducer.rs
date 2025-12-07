//! Add Repository Form Reducer
//!
//! Handles state changes for the add repository form.

use crate::actions::AddRepositoryAction;
use crate::state::{AddRepoField, AddRepoFormState};

/// Reduce add repository form state based on actions.
///
/// Accepts only AddRepositoryAction, making it type-safe and focused.
pub fn reduce_add_repository(
    mut state: AddRepoFormState,
    action: &AddRepositoryAction,
) -> AddRepoFormState {
    match action {
        AddRepositoryAction::Char(c) => {
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

        AddRepositoryAction::Backspace => {
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

        AddRepositoryAction::ClearField => {
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

        AddRepositoryAction::NextField => {
            state.focused_field = state.focused_field.next();
        }

        AddRepositoryAction::PrevField => {
            state.focused_field = state.focused_field.prev();
        }

        // Confirm and Close: form reset handled in app_reducer,
        // view management handled in add_repository middleware
        AddRepositoryAction::Confirm | AddRepositoryAction::Close => {
            // No state changes needed here - handled by app_reducer
        }
    }

    state
}
