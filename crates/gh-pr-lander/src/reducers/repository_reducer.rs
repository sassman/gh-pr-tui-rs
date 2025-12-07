use crate::actions::RepositoryAction;
use crate::state::MainViewState;

/// TODO: MainViewState is a misleading name here - it should be RepositoryViewState
pub fn reduce_repository(mut state: MainViewState, action: &RepositoryAction) -> MainViewState {
    match action {
        RepositoryAction::OpenRepositoryInBrowser => todo!(),
        RepositoryAction::AddRepository(repo) => {
            log::info!("Adding repository: {}", repo.display_name());
            state.repositories.push(repo.clone());
        }
        RepositoryAction::LoadRepositoryData(_) => {
            // that is a side effect handled by middleware
        }
    }
    state
}
