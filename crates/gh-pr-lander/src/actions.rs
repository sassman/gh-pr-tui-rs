use ratatui::crossterm::event::KeyEvent;

use crate::{
    domain_models::{Pr, Repository},
    logger::OwnedLogRecord,
    views::View,
};

/// Actions represent all possible state changes in the application.
/// Actions are prefixed by scope to indicate which part of the app they affect.
pub enum Action {
    /// Global actions (not tied to any specific view)
    GlobalKeyPressed(KeyEvent),
    GlobalClose,
    GlobalQuit,

    /// ## View stack management
    /// Push a new view onto the stack (for modals/popups)
    PushView(Box<dyn View>),
    /// Replace entire view stack with new view (for navigation)
    ReplaceView(Box<dyn View>),

    /// ## Local actions (dispatched to active view for handling)
    /// Key pressed in active view context
    LocalKeyPressed(char),

    /// ## Navigation actions (semantic, vim-style)
    NavigateNext, // j, down arrow
    NavigatePrevious, // k, up arrow
    NavigateLeft,     // h, left arrow
    NavigateRight,    // l, right arrow

    /// ## Repository navigation actions
    RepositoryNext, // Switch to next repository
    RepositoryPrevious, // Switch to previous repository

    /// ## Scroll actions
    ScrollToTop, // gg
    ScrollToBottom,     // G
    ScrollPageDown,     // Page Down
    ScrollPageUp,       // Page Up
    ScrollHalfPageDown, // Ctrl+d
    ScrollHalfPageUp,   // Ctrl+u

    /// ## Debug console actions
    DebugConsoleClear, // Clear debug console logs
    DebugConsoleLogAdded(OwnedLogRecord), // New log record added
    DebugConsoleDumpLogs,

    /// ## Text input actions (generic, for any view with TEXT_INPUT capability)
    TextInputChar(char), // Character typed into input field
    TextInputBackspace, // Backspace pressed in input field
    TextInputClearLine, // Cmd+Backspace - clear entire field/line
    TextInputEscape,    // Escape pressed in input field (clear or close)
    TextInputConfirm,   // Enter pressed in input field (confirm/execute)

    /// ## Command palette actions
    CommandPaletteChar(char), // Character typed into search field
    CommandPaletteBackspace,    // Backspace pressed in search field
    CommandPaletteClear,        // Clear entire query (Cmd+Backspace)
    CommandPaletteClose,        // Close the command palette
    CommandPaletteExecute,      // Execute selected command
    CommandPaletteNavigateNext, // Navigate to next command
    CommandPaletteNavigatePrev, // Navigate to previous command

    /// ## Repository management actions
    RepositoryAdd, // Show add repository dialog/popup
    RepositoryAddBulk(Vec<Repository>), // Add multiple repositories at once (from config)

    /// ## Add repository form actions
    AddRepoChar(char), // Character typed into current field
    AddRepoBackspace,  // Backspace pressed in current field
    AddRepoClearField, // Clear entire current field (Cmd+Backspace)
    AddRepoNextField,  // Move to next field (Tab)
    AddRepoPrevField,  // Move to previous field (Shift+Tab)
    AddRepoConfirm,    // Confirm and add the repository (Enter)
    AddRepoClose,      // Close the form without adding (Esc)

    /// ## Pull Request actions
    /// Start loading PRs for a repository (repo_index)
    PrLoadStart(usize),
    /// PRs loaded successfully for a repository (repo_index, prs)
    PrLoaded(usize, Vec<Pr>),
    /// Failed to load PRs for a repository (repo_index, error_message)
    PrLoadError(usize, String),
    /// Navigate to next PR in the table
    PrNavigateNext,
    /// Navigate to previous PR in the table
    PrNavigatePrevious,
    /// Refresh PRs for the current repository
    PrRefresh,

    /// ## PR Selection actions (for bulk operations)
    /// Toggle selection of the current PR (at cursor)
    PrToggleSelection,
    /// Select all PRs in the current repository
    PrSelectAll,
    /// Deselect all PRs in the current repository
    PrDeselectAll,

    /// ## PR Operations
    /// Open current PR in browser
    PrOpenInBrowser,
    /// Request to merge selected PRs (or cursor PR if none selected)
    PrMergeRequest,
    /// Merge started for a PR
    PrMergeStart(usize, usize), // repo_idx, pr_number
    /// Merge succeeded
    PrMergeSuccess(usize, usize), // repo_idx, pr_number
    /// Merge failed
    PrMergeError(usize, usize, String), // repo_idx, pr_number, error
    /// Request to rebase/update selected PRs
    PrRebaseRequest,
    /// Rebase started for a PR
    PrRebaseStart(usize, usize), // repo_idx, pr_number
    /// Rebase succeeded
    PrRebaseSuccess(usize, usize), // repo_idx, pr_number
    /// Rebase failed
    PrRebaseError(usize, usize, String), // repo_idx, pr_number, error
    /// Request to approve selected PRs
    PrApproveRequest,
    /// Approve started for a PR
    PrApproveStart(usize, usize), // repo_idx, pr_number
    /// Approve succeeded
    PrApproveSuccess(usize, usize), // repo_idx, pr_number
    /// Approve failed
    PrApproveError(usize, usize, String), // repo_idx, pr_number, error
    /// Request to close selected PRs
    PrCloseRequest,
    /// Close started for a PR
    PrCloseStart(usize, usize), // repo_idx, pr_number
    /// Close succeeded
    PrCloseSuccess(usize, usize), // repo_idx, pr_number
    /// Close failed
    PrCloseError(usize, usize, String), // repo_idx, pr_number, error

    /// ## CI/Build Status actions
    /// Request to rerun failed jobs for the current PR
    PrRerunFailedJobs,
    /// Rerun started for a workflow run
    PrRerunStart(usize, u64, u64), // repo_idx, pr_number, run_id
    /// Rerun succeeded
    PrRerunSuccess(usize, u64, u64), // repo_idx, pr_number, run_id
    /// Rerun failed
    PrRerunError(usize, u64, u64, String), // repo_idx, pr_number, run_id, error
    /// Open CI build logs in browser (for current PR)
    PrOpenBuildLogs,

    /// ## IDE Integration actions
    /// Open current PR diff in configured IDE (uses gh CLI under the hood)
    PrOpenInIDE,

    /// ## Bootstrap actions
    BootstrapStart,
    BootstrapEnd,

    /// ## Repository loading actions
    /// Load recent repositories from config (dispatched by bootstrap)
    LoadRecentRepositories,
    /// Recent repositories loaded (dispatched by repository middleware)
    LoadRecentRepositoriesDone,

    /// ## Animation/Timer actions
    Tick, // Periodic tick for animations (500ms interval)

    ///No-op action
    None,
}

impl Clone for Action {
    fn clone(&self) -> Self {
        match self {
            Self::GlobalKeyPressed(key) => Self::GlobalKeyPressed(*key),
            Self::GlobalClose => Self::GlobalClose,
            Self::GlobalQuit => Self::GlobalQuit,
            Self::PushView(view) => Self::PushView(view.clone()),
            Self::ReplaceView(view) => Self::ReplaceView(view.clone()),
            Self::LocalKeyPressed(c) => Self::LocalKeyPressed(*c),
            Self::NavigateNext => Self::NavigateNext,
            Self::NavigatePrevious => Self::NavigatePrevious,
            Self::NavigateLeft => Self::NavigateLeft,
            Self::NavigateRight => Self::NavigateRight,
            Self::RepositoryNext => Self::RepositoryNext,
            Self::RepositoryPrevious => Self::RepositoryPrevious,
            Self::ScrollToTop => Self::ScrollToTop,
            Self::ScrollToBottom => Self::ScrollToBottom,
            Self::ScrollPageDown => Self::ScrollPageDown,
            Self::ScrollPageUp => Self::ScrollPageUp,
            Self::ScrollHalfPageDown => Self::ScrollHalfPageDown,
            Self::ScrollHalfPageUp => Self::ScrollHalfPageUp,
            Self::DebugConsoleClear => Self::DebugConsoleClear,
            Self::DebugConsoleLogAdded(record) => Self::DebugConsoleLogAdded(record.clone()),
            Self::DebugConsoleDumpLogs => Self::DebugConsoleDumpLogs,
            Self::TextInputChar(c) => Self::TextInputChar(*c),
            Self::TextInputBackspace => Self::TextInputBackspace,
            Self::TextInputClearLine => Self::TextInputClearLine,
            Self::TextInputEscape => Self::TextInputEscape,
            Self::TextInputConfirm => Self::TextInputConfirm,
            Self::CommandPaletteChar(c) => Self::CommandPaletteChar(*c),
            Self::CommandPaletteBackspace => Self::CommandPaletteBackspace,
            Self::CommandPaletteClear => Self::CommandPaletteClear,
            Self::CommandPaletteClose => Self::CommandPaletteClose,
            Self::CommandPaletteExecute => Self::CommandPaletteExecute,
            Self::CommandPaletteNavigateNext => Self::CommandPaletteNavigateNext,
            Self::CommandPaletteNavigatePrev => Self::CommandPaletteNavigatePrev,
            Self::RepositoryAdd => Self::RepositoryAdd,
            Self::RepositoryAddBulk(repos) => Self::RepositoryAddBulk(repos.clone()),
            Self::AddRepoChar(c) => Self::AddRepoChar(*c),
            Self::AddRepoBackspace => Self::AddRepoBackspace,
            Self::AddRepoClearField => Self::AddRepoClearField,
            Self::AddRepoNextField => Self::AddRepoNextField,
            Self::AddRepoPrevField => Self::AddRepoPrevField,
            Self::AddRepoConfirm => Self::AddRepoConfirm,
            Self::AddRepoClose => Self::AddRepoClose,
            Self::PrLoadStart(idx) => Self::PrLoadStart(*idx),
            Self::PrLoaded(idx, prs) => Self::PrLoaded(*idx, prs.clone()),
            Self::PrLoadError(idx, err) => Self::PrLoadError(*idx, err.clone()),
            Self::PrNavigateNext => Self::PrNavigateNext,
            Self::PrNavigatePrevious => Self::PrNavigatePrevious,
            Self::PrRefresh => Self::PrRefresh,
            Self::PrToggleSelection => Self::PrToggleSelection,
            Self::PrSelectAll => Self::PrSelectAll,
            Self::PrDeselectAll => Self::PrDeselectAll,
            Self::PrOpenInBrowser => Self::PrOpenInBrowser,
            Self::PrMergeRequest => Self::PrMergeRequest,
            Self::PrMergeStart(repo, pr) => Self::PrMergeStart(*repo, *pr),
            Self::PrMergeSuccess(repo, pr) => Self::PrMergeSuccess(*repo, *pr),
            Self::PrMergeError(repo, pr, err) => Self::PrMergeError(*repo, *pr, err.clone()),
            Self::PrRebaseRequest => Self::PrRebaseRequest,
            Self::PrRebaseStart(repo, pr) => Self::PrRebaseStart(*repo, *pr),
            Self::PrRebaseSuccess(repo, pr) => Self::PrRebaseSuccess(*repo, *pr),
            Self::PrRebaseError(repo, pr, err) => Self::PrRebaseError(*repo, *pr, err.clone()),
            Self::PrApproveRequest => Self::PrApproveRequest,
            Self::PrApproveStart(repo, pr) => Self::PrApproveStart(*repo, *pr),
            Self::PrApproveSuccess(repo, pr) => Self::PrApproveSuccess(*repo, *pr),
            Self::PrApproveError(repo, pr, err) => Self::PrApproveError(*repo, *pr, err.clone()),
            Self::PrCloseRequest => Self::PrCloseRequest,
            Self::PrCloseStart(repo, pr) => Self::PrCloseStart(*repo, *pr),
            Self::PrCloseSuccess(repo, pr) => Self::PrCloseSuccess(*repo, *pr),
            Self::PrCloseError(repo, pr, err) => Self::PrCloseError(*repo, *pr, err.clone()),
            Self::PrRerunFailedJobs => Self::PrRerunFailedJobs,
            Self::PrRerunStart(repo, pr, run) => Self::PrRerunStart(*repo, *pr, *run),
            Self::PrRerunSuccess(repo, pr, run) => Self::PrRerunSuccess(*repo, *pr, *run),
            Self::PrRerunError(repo, pr, run, err) => {
                Self::PrRerunError(*repo, *pr, *run, err.clone())
            }
            Self::PrOpenBuildLogs => Self::PrOpenBuildLogs,
            Self::PrOpenInIDE => Self::PrOpenInIDE,
            Self::BootstrapStart => Self::BootstrapStart,
            Self::BootstrapEnd => Self::BootstrapEnd,
            Self::LoadRecentRepositories => Self::LoadRecentRepositories,
            Self::LoadRecentRepositoriesDone => Self::LoadRecentRepositoriesDone,
            Self::Tick => Self::Tick,
            Self::None => Self::None,
        }
    }
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GlobalKeyPressed(key) => f.debug_tuple("GlobalKeyPressed").field(key).finish(),
            Self::GlobalClose => write!(f, "GlobalClose"),
            Self::GlobalQuit => write!(f, "GlobalQuit"),
            Self::PushView(view) => f.debug_tuple("PushView").field(view).finish(),
            Self::ReplaceView(view) => f.debug_tuple("ReplaceView").field(view).finish(),
            Self::LocalKeyPressed(c) => f.debug_tuple("LocalKeyPressed").field(c).finish(),
            Self::NavigateNext => write!(f, "NavigateNext"),
            Self::NavigatePrevious => write!(f, "NavigatePrevious"),
            Self::NavigateLeft => write!(f, "NavigateLeft"),
            Self::NavigateRight => write!(f, "NavigateRight"),
            Self::RepositoryNext => write!(f, "RepositoryNext"),
            Self::RepositoryPrevious => write!(f, "RepositoryPrevious"),
            Self::ScrollToTop => write!(f, "ScrollToTop"),
            Self::ScrollToBottom => write!(f, "ScrollToBottom"),
            Self::ScrollPageDown => write!(f, "ScrollPageDown"),
            Self::ScrollPageUp => write!(f, "ScrollPageUp"),
            Self::ScrollHalfPageDown => write!(f, "ScrollHalfPageDown"),
            Self::ScrollHalfPageUp => write!(f, "ScrollHalfPageUp"),
            Self::DebugConsoleClear => write!(f, "DebugConsoleClear"),
            Self::DebugConsoleLogAdded(record) => {
                f.debug_tuple("DebugConsoleLogAdded").field(record).finish()
            }
            Self::DebugConsoleDumpLogs => write!(f, "DebugConsoleDumpLogs"),
            Self::TextInputChar(c) => f.debug_tuple("TextInputChar").field(c).finish(),
            Self::TextInputBackspace => write!(f, "TextInputBackspace"),
            Self::TextInputClearLine => write!(f, "TextInputClearLine"),
            Self::TextInputEscape => write!(f, "TextInputEscape"),
            Self::TextInputConfirm => write!(f, "TextInputConfirm"),
            Self::CommandPaletteChar(c) => f.debug_tuple("CommandPaletteChar").field(c).finish(),
            Self::CommandPaletteBackspace => write!(f, "CommandPaletteBackspace"),
            Self::CommandPaletteClear => write!(f, "CommandPaletteClear"),
            Self::CommandPaletteClose => write!(f, "CommandPaletteClose"),
            Self::CommandPaletteExecute => write!(f, "CommandPaletteExecute"),
            Self::CommandPaletteNavigateNext => write!(f, "CommandPaletteNavigateNext"),
            Self::CommandPaletteNavigatePrev => write!(f, "CommandPaletteNavigatePrev"),
            Self::RepositoryAdd => write!(f, "RepositoryAdd"),
            Self::RepositoryAddBulk(repos) => {
                write!(f, "RepositoryAddBulk({} repos)", repos.len())
            }
            Self::AddRepoChar(c) => f.debug_tuple("AddRepoChar").field(c).finish(),
            Self::AddRepoBackspace => write!(f, "AddRepoBackspace"),
            Self::AddRepoClearField => write!(f, "AddRepoClearField"),
            Self::AddRepoNextField => write!(f, "AddRepoNextField"),
            Self::AddRepoPrevField => write!(f, "AddRepoPrevField"),
            Self::AddRepoConfirm => write!(f, "AddRepoConfirm"),
            Self::AddRepoClose => write!(f, "AddRepoClose"),
            Self::PrLoadStart(idx) => write!(f, "PrLoadStart({})", idx),
            Self::PrLoaded(idx, prs) => write!(f, "PrLoaded({}, {} prs)", idx, prs.len()),
            Self::PrLoadError(idx, err) => write!(f, "PrLoadError({}, {})", idx, err),
            Self::PrNavigateNext => write!(f, "PrNavigateNext"),
            Self::PrNavigatePrevious => write!(f, "PrNavigatePrevious"),
            Self::PrRefresh => write!(f, "PrRefresh"),
            Self::PrToggleSelection => write!(f, "PrToggleSelection"),
            Self::PrSelectAll => write!(f, "PrSelectAll"),
            Self::PrDeselectAll => write!(f, "PrDeselectAll"),
            Self::PrOpenInBrowser => write!(f, "PrOpenInBrowser"),
            Self::PrMergeRequest => write!(f, "PrMergeRequest"),
            Self::PrMergeStart(repo, pr) => write!(f, "PrMergeStart({}, #{})", repo, pr),
            Self::PrMergeSuccess(repo, pr) => write!(f, "PrMergeSuccess({}, #{})", repo, pr),
            Self::PrMergeError(repo, pr, err) => {
                write!(f, "PrMergeError({}, #{}, {})", repo, pr, err)
            }
            Self::PrRebaseRequest => write!(f, "PrRebaseRequest"),
            Self::PrRebaseStart(repo, pr) => write!(f, "PrRebaseStart({}, #{})", repo, pr),
            Self::PrRebaseSuccess(repo, pr) => write!(f, "PrRebaseSuccess({}, #{})", repo, pr),
            Self::PrRebaseError(repo, pr, err) => {
                write!(f, "PrRebaseError({}, #{}, {})", repo, pr, err)
            }
            Self::PrApproveRequest => write!(f, "PrApproveRequest"),
            Self::PrApproveStart(repo, pr) => write!(f, "PrApproveStart({}, #{})", repo, pr),
            Self::PrApproveSuccess(repo, pr) => write!(f, "PrApproveSuccess({}, #{})", repo, pr),
            Self::PrApproveError(repo, pr, err) => {
                write!(f, "PrApproveError({}, #{}, {})", repo, pr, err)
            }
            Self::PrCloseRequest => write!(f, "PrCloseRequest"),
            Self::PrCloseStart(repo, pr) => write!(f, "PrCloseStart({}, #{})", repo, pr),
            Self::PrCloseSuccess(repo, pr) => write!(f, "PrCloseSuccess({}, #{})", repo, pr),
            Self::PrCloseError(repo, pr, err) => {
                write!(f, "PrCloseError({}, #{}, {})", repo, pr, err)
            }
            Self::PrRerunFailedJobs => write!(f, "PrRerunFailedJobs"),
            Self::PrRerunStart(repo, pr, run) => {
                write!(f, "PrRerunStart({}, #{}, run={})", repo, pr, run)
            }
            Self::PrRerunSuccess(repo, pr, run) => {
                write!(f, "PrRerunSuccess({}, #{}, run={})", repo, pr, run)
            }
            Self::PrRerunError(repo, pr, run, err) => {
                write!(f, "PrRerunError({}, #{}, run={}, {})", repo, pr, run, err)
            }
            Self::PrOpenBuildLogs => write!(f, "PrOpenBuildLogs"),
            Self::PrOpenInIDE => write!(f, "PrOpenInIDE"),
            Self::BootstrapStart => write!(f, "BootstrapStart"),
            Self::BootstrapEnd => write!(f, "BootstrapEnd"),
            Self::LoadRecentRepositories => write!(f, "LoadRecentRepositories"),
            Self::LoadRecentRepositoriesDone => write!(f, "LoadRecentRepositoriesDone"),
            Self::Tick => write!(f, "Tick"),
            Self::None => write!(f, "None"),
        }
    }
}
