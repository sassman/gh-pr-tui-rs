//! Actions for the Repository view.

use crate::domain_models::Repository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryAction {
    /// Open the current repository in the browser
    OpenRepositoryInBrowser,

    /// Adds a new repository
    AddRepository(Repository),

    /// Load all repository related data (e.g., pull requests etc.)
    LoadRepositoryData(Repository),
}
