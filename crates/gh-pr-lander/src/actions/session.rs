//! Session actions
//!
//! Actions for session state management (load/save/restore).

/// Actions for session lifecycle management
#[derive(Debug, Clone)]
pub enum SessionAction {
    /// Session state loaded from disk
    /// Contains the previously selected repository and PR number
    Loaded {
        /// Selected repository as (org, name, branch, host)
        /// host is None for github.com repositories
        selected_repo: Option<(String, String, String, Option<String>)>,
        /// Selected PR number (not index)
        selected_pr_no: Option<usize>,
    },

    /// Request to restore session selection after repositories are loaded
    /// This is triggered when repositories finish loading
    RestoreSelection,
}
