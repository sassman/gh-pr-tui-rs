//! Command identifiers
//!
//! This module defines all command IDs as an enum for type-safe,
//! memory-efficient command references that can be serialized/deserialized.

use serde::{Deserialize, Serialize};
use strum::EnumIter;

use crate::{actions::RepositoryAction, views::KeyBindingsView};

/// Unique identifier for each command in the application.
///
/// Commands are the semantic actions users can trigger. Each command
/// has a unique ID that can be referenced in keybindings and the command palette.
///
/// The enum is serialized as snake_case (e.g., `RepositoryAdd` -> `"repository_add"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum CommandId {
    // === Semantic/Context actions (translated by views) ===
    /// Primary action on focused item (Enter key)
    /// Views interpret this differently (open, execute, submit, etc.)
    Confirm,
    /// Toggle selection/state of focused item (Space key)
    ToggleSelect,
    /// Select all items (Ctrl+A)
    SelectAll,
    /// Deselect all items
    DeselectAll,

    // === Repository management ===
    /// Add a new repository to track
    RepositoryAdd,
    /// Remove the current repository from the list
    RepositoryRemove,
    /// Open the current repository in the browser
    RepositoryOpenInBrowser,
    /// Switch to the next repository
    RepositoryNext,
    /// Switch to the previous repository
    RepositoryPrevious,

    // === Navigation ===
    /// Navigate to the next item (down)
    NavigateNext,
    /// Navigate to the previous item (up)
    NavigatePrevious,
    /// Navigate left
    NavigateLeft,
    /// Navigate right
    NavigateRight,
    /// Navigate to the top (gg in vim)
    NavigateToTop,
    /// Navigate to the bottom (G in vim)
    NavigateToBottom,

    // === Debug ===
    /// Toggle the debug console visibility
    DebugToggleConsoleView,
    /// Clear the debug console logs
    DebugClearLogs,

    // === Command palette ===
    /// Open the command palette
    CommandPaletteOpen,

    // === PR Selection ===
    /// Toggle selection of current PR
    PrToggleSelection,
    /// Select all PRs in current repository
    PrSelectAll,
    /// Deselect all PRs
    PrDeselectAll,
    /// Refresh PRs for current repository
    PrRefresh,

    // === PR Operations ===
    /// Open current PR in browser
    PrOpenInBrowser,
    /// Merge selected PRs
    PrMerge,
    /// Rebase/update selected PRs
    PrRebase,
    /// Approve selected PRs (shows confirmation popup)
    PrApprove,
    /// Comment on selected PRs (shows confirmation popup)
    PrComment,
    /// Request changes on selected PRs (shows confirmation popup)
    PrRequestChanges,
    /// Close selected PRs without merging (shows confirmation popup)
    PrClose,

    // === CI/Build Status ===
    /// Rerun failed CI jobs for current PR
    PrRerunFailedJobs,
    /// Open CI build logs in browser
    PrOpenBuildLogs,

    // === IDE Integration ===
    /// Open current PR in configured IDE
    PrOpenInIDE,

    // === Filter & Search ===
    /// Cycle through filter presets
    PrCycleFilter,
    /// Clear the current filter
    PrClearFilter,

    // === Merge Bot ===
    /// Start merge bot for selected PRs
    MergeBotStart,
    /// Stop the merge bot
    MergeBotStop,
    /// Add PRs to merge queue
    MergeBotAddToQueue,

    // === Help ===
    /// Toggle key bindings help panel
    KeyBindingsToggleView,

    // === Build Log ===
    /// Open build logs viewer for current PR
    BuildLogOpen,
    /// Navigate to next error in build logs
    BuildLogNextError,
    /// Navigate to previous error in build logs
    BuildLogPrevError,
    /// Toggle expand/collapse in build logs
    BuildLogToggle,
    /// Toggle timestamps in build logs
    BuildLogToggleTimestamps,
    /// Expand all nodes in build logs
    BuildLogExpandAll,
    /// Collapse all nodes in build logs
    BuildLogCollapseAll,

    // === Diff Viewer ===
    /// Open diff viewer for current PR
    DiffViewerOpen,
    /// Switch focus between file tree and diff content pane
    DiffViewerSwitchPane,
    /// Add a comment on the current line
    DiffViewerAddComment,
    /// Submit the current comment
    DiffViewerSubmitComment,
    /// Cancel comment editing
    DiffViewerCancelComment,
    /// Enter visual mode for line selection
    DiffViewerVisualMode,
    /// Show review submission popup
    DiffViewerShowReviewPopup,
    /// Page down in diff viewer
    DiffViewerPageDown,
    /// Page up in diff viewer
    DiffViewerPageUp,

    // === General ===
    /// Close the current view/panel
    GlobalClose,
    /// Quit the application
    GlobalQuit,
}

impl CommandId {
    /// Convert this command ID to an Action
    ///
    /// Some commands require context (like views) to create their action,
    /// those are handled separately in the reducer.
    pub fn to_action(self) -> crate::actions::Action {
        use crate::actions::{
            Action, ContextAction, DebugConsoleAction, GlobalAction, MergeBotAction,
            NavigationAction, PullRequestAction,
        };
        use crate::views::{AddRepositoryView, CommandPaletteView, DebugConsoleView};

        match self {
            // Semantic/Context actions (translated by views)
            Self::Confirm => Action::ViewContext(ContextAction::Confirm),
            Self::ToggleSelect => Action::ViewContext(ContextAction::ToggleSelect),
            Self::SelectAll => Action::ViewContext(ContextAction::SelectAll),
            Self::DeselectAll => Action::ViewContext(ContextAction::DeselectAll),

            // Repository
            Self::RepositoryAdd => {
                Action::Global(GlobalAction::PushView(Box::new(AddRepositoryView::new())))
            }
            Self::RepositoryRemove => Action::Repository(RepositoryAction::RemoveCurrentRepository),
            Self::RepositoryOpenInBrowser => {
                Action::Repository(RepositoryAction::OpenRepositoryInBrowser)
            }
            Self::RepositoryNext => Action::PullRequest(PullRequestAction::RepositoryNext),
            Self::RepositoryPrevious => Action::PullRequest(PullRequestAction::RepositoryPrevious),

            // Navigation
            Self::NavigateNext => Action::Navigate(NavigationAction::Next),
            Self::NavigatePrevious => Action::Navigate(NavigationAction::Previous),
            Self::NavigateLeft => Action::Navigate(NavigationAction::Left),
            Self::NavigateRight => Action::Navigate(NavigationAction::Right),
            Self::NavigateToTop => Action::Navigate(NavigationAction::ToTop),
            Self::NavigateToBottom => Action::Navigate(NavigationAction::ToBottom),

            // Debug
            Self::DebugToggleConsoleView => {
                Action::Global(GlobalAction::PushView(Box::new(DebugConsoleView::new())))
            }
            Self::DebugClearLogs => Action::DebugConsole(DebugConsoleAction::Clear),

            // Command palette
            Self::CommandPaletteOpen => {
                Action::Global(GlobalAction::PushView(Box::new(CommandPaletteView::new())))
            }

            // PR Selection
            Self::PrToggleSelection => Action::PullRequest(PullRequestAction::ToggleSelection),
            Self::PrSelectAll => Action::PullRequest(PullRequestAction::SelectAll),
            Self::PrDeselectAll => Action::PullRequest(PullRequestAction::DeselectAll),
            Self::PrRefresh => Action::PullRequest(PullRequestAction::Refresh),

            // PR Operations
            Self::PrOpenInBrowser => Action::PullRequest(PullRequestAction::OpenInBrowser),
            Self::PrMerge => Action::PullRequest(PullRequestAction::MergeRequest),
            Self::PrRebase => Action::PullRequest(PullRequestAction::RebaseRequest),
            Self::PrApprove => Action::PullRequest(PullRequestAction::ApproveRequest),
            Self::PrComment => Action::PullRequest(PullRequestAction::CommentRequest),
            Self::PrRequestChanges => Action::PullRequest(PullRequestAction::RequestChangesRequest),
            Self::PrClose => Action::PullRequest(PullRequestAction::CloseRequest),

            // CI/Build Status
            Self::PrRerunFailedJobs => Action::PullRequest(PullRequestAction::RerunFailedJobs),
            Self::PrOpenBuildLogs => Action::PullRequest(PullRequestAction::OpenBuildLogs),

            // IDE Integration
            Self::PrOpenInIDE => Action::PullRequest(PullRequestAction::OpenInIDE),

            // Filter & Search
            Self::PrCycleFilter => Action::PullRequest(PullRequestAction::CycleFilter),
            Self::PrClearFilter => Action::PullRequest(PullRequestAction::ClearFilter),

            // Merge Bot
            Self::MergeBotStart => Action::MergeBot(MergeBotAction::Start),
            Self::MergeBotStop => Action::MergeBot(MergeBotAction::Stop),
            Self::MergeBotAddToQueue => Action::MergeBot(MergeBotAction::AddToQueue),

            // Help
            Self::KeyBindingsToggleView => {
                Action::Global(GlobalAction::PushView(Box::new(KeyBindingsView::new())))
            }

            // Build Log
            Self::BuildLogOpen => Action::BuildLog(crate::actions::BuildLogAction::Open),
            Self::BuildLogNextError => Action::BuildLog(crate::actions::BuildLogAction::NextError),
            Self::BuildLogPrevError => Action::BuildLog(crate::actions::BuildLogAction::PrevError),
            Self::BuildLogToggle => Action::BuildLog(crate::actions::BuildLogAction::Toggle),
            Self::BuildLogToggleTimestamps => {
                Action::BuildLog(crate::actions::BuildLogAction::ToggleTimestamps)
            }
            Self::BuildLogExpandAll => Action::BuildLog(crate::actions::BuildLogAction::ExpandAll),
            Self::BuildLogCollapseAll => {
                Action::BuildLog(crate::actions::BuildLogAction::CollapseAll)
            }

            // Diff Viewer
            Self::DiffViewerOpen => Action::DiffViewer(crate::actions::DiffViewerAction::Open),
            Self::DiffViewerSwitchPane => {
                Action::DiffViewer(crate::actions::DiffViewerAction::SwitchPane)
            }
            Self::DiffViewerAddComment => {
                Action::DiffViewer(crate::actions::DiffViewerAction::AddComment)
            }
            Self::DiffViewerSubmitComment => {
                Action::DiffViewer(crate::actions::DiffViewerAction::CommitComment)
            }
            Self::DiffViewerCancelComment => {
                Action::DiffViewer(crate::actions::DiffViewerAction::CancelComment)
            }
            Self::DiffViewerVisualMode => {
                Action::DiffViewer(crate::actions::DiffViewerAction::EnterVisualMode)
            }
            Self::DiffViewerShowReviewPopup => {
                Action::DiffViewer(crate::actions::DiffViewerAction::ShowReviewPopup)
            }
            Self::DiffViewerPageDown => {
                Action::DiffViewer(crate::actions::DiffViewerAction::PageDown)
            }
            Self::DiffViewerPageUp => Action::DiffViewer(crate::actions::DiffViewerAction::PageUp),

            // General
            Self::GlobalClose => Action::Global(GlobalAction::Close),
            Self::GlobalQuit => Action::Global(GlobalAction::Quit),
        }
    }

    /// Get the default title for this command (used in command palette)
    pub fn title(&self) -> &'static str {
        match self {
            // Semantic/Context actions
            Self::Confirm => "Confirm / Primary action",
            Self::ToggleSelect => "Toggle selection",
            Self::SelectAll => "Select all",
            Self::DeselectAll => "Deselect all",

            // Repository
            Self::RepositoryAdd => "Add repository",
            Self::RepositoryRemove => "Remove repository",
            Self::RepositoryOpenInBrowser => "Open repository in browser",
            Self::RepositoryNext => "Next repository",
            Self::RepositoryPrevious => "Previous repository",

            // Navigation
            Self::NavigateNext => "Navigate down",
            Self::NavigatePrevious => "Navigate up",
            Self::NavigateLeft => "Navigate left",
            Self::NavigateRight => "Navigate right",
            Self::NavigateToTop => "Navigate to top",
            Self::NavigateToBottom => "Navigate to bottom",

            // Debug
            Self::DebugToggleConsoleView => "Toggle debug console",
            Self::DebugClearLogs => "Clear debug logs",

            // Command palette
            Self::CommandPaletteOpen => "Open command palette",

            // PR Selection
            Self::PrToggleSelection => "Toggle PR selection",
            Self::PrSelectAll => "Select all PRs",
            Self::PrDeselectAll => "Deselect all PRs",
            Self::PrRefresh => "Refresh PRs",

            // PR Operations
            Self::PrOpenInBrowser => "Open PR in browser",
            Self::PrMerge => "Merge PRs",
            Self::PrRebase => "Rebase PRs",
            Self::PrApprove => "Approve PRs",
            Self::PrComment => "Comment on PRs",
            Self::PrRequestChanges => "Request changes on PRs",
            Self::PrClose => "Close PRs",

            // CI/Build Status
            Self::PrRerunFailedJobs => "Rerun failed CI jobs",
            Self::PrOpenBuildLogs => "Open CI build logs",

            // IDE Integration
            Self::PrOpenInIDE => "Open PR diff in IDE",

            // Filter & Search
            Self::PrCycleFilter => "Cycle PR filter",
            Self::PrClearFilter => "Clear PR filter",

            // Merge Bot
            Self::MergeBotStart => "Start merge bot",
            Self::MergeBotStop => "Stop merge bot",
            Self::MergeBotAddToQueue => "Add to merge queue",

            // Help
            Self::KeyBindingsToggleView => "Show key bindings",

            // Build Log
            Self::BuildLogOpen => "Open build logs",
            Self::BuildLogNextError => "Next error",
            Self::BuildLogPrevError => "Previous error",
            Self::BuildLogToggle => "Toggle expand/collapse",
            Self::BuildLogToggleTimestamps => "Toggle timestamps",
            Self::BuildLogExpandAll => "Expand all",
            Self::BuildLogCollapseAll => "Collapse all",

            // Diff Viewer
            Self::DiffViewerOpen => "Open diff viewer",
            Self::DiffViewerSwitchPane => "Switch pane",
            Self::DiffViewerAddComment => "Add comment",
            Self::DiffViewerSubmitComment => "Submit comment",
            Self::DiffViewerCancelComment => "Cancel comment",
            Self::DiffViewerVisualMode => "Visual mode",
            Self::DiffViewerShowReviewPopup => "Submit review",
            Self::DiffViewerPageDown => "Page down",
            Self::DiffViewerPageUp => "Page up",

            // General
            Self::GlobalClose => "Close",
            Self::GlobalQuit => "Quit",
        }
    }

    /// Get the default description for this command
    pub fn description(&self) -> &'static str {
        match self {
            // Semantic/Context actions
            Self::Confirm => "Execute primary action on focused item (context-dependent)",
            Self::ToggleSelect => "Toggle selection or state of focused item",
            Self::SelectAll => "Select all items in the current view",
            Self::DeselectAll => "Clear all selections",

            // Repository
            Self::RepositoryAdd => "Add a new repository to track",
            Self::RepositoryRemove => "Remove the current repository from the list",
            Self::RepositoryOpenInBrowser => "Open the current repository in your browser",
            Self::RepositoryNext => "Switch to the next repository",
            Self::RepositoryPrevious => "Switch to the previous repository",

            // Navigation
            Self::NavigateNext => "Move selection or navigate down",
            Self::NavigatePrevious => "Move selection or navigate up",
            Self::NavigateLeft => "Move selection or navigate left",
            Self::NavigateRight => "Move selection or navigate right",
            Self::NavigateToTop => "Jump to the first item",
            Self::NavigateToBottom => "Jump to the last item",

            // Debug
            Self::DebugToggleConsoleView => "Show or hide the debug console",
            Self::DebugClearLogs => "Clear all debug console logs",

            // Command palette
            Self::CommandPaletteOpen => "Open the command palette to search and execute commands",

            // PR Selection
            Self::PrToggleSelection => "Toggle selection of the current PR for bulk operations",
            Self::PrSelectAll => "Select all PRs in the current repository",
            Self::PrDeselectAll => "Clear all PR selections",
            Self::PrRefresh => "Refresh PRs for the current repository",

            // PR Operations
            Self::PrOpenInBrowser => "Open the current PR in your default web browser",
            Self::PrMerge => "Merge selected PRs (or current PR if none selected)",
            Self::PrRebase => "Update selected PRs with latest from base branch",
            Self::PrApprove => "Approve selected PRs with a review",
            Self::PrComment => "Post a comment on selected PRs",
            Self::PrRequestChanges => "Request changes on selected PRs with a review",
            Self::PrClose => "Close selected PRs without merging",

            // CI/Build Status
            Self::PrRerunFailedJobs => "Rerun failed CI workflow jobs for the current PR",
            Self::PrOpenBuildLogs => "Open CI build logs in your default web browser",

            // IDE Integration
            Self::PrOpenInIDE => "Open the PR diff in your configured IDE (uses gh pr view)",

            // Filter & Search
            Self::PrCycleFilter => "Cycle through filter presets (All, Ready, Needs Rebase, etc.)",
            Self::PrClearFilter => "Clear the current filter and show all PRs",

            // Merge Bot
            Self::MergeBotStart => "Start automated merge bot for selected PRs",
            Self::MergeBotStop => "Stop the merge bot and clear the queue",
            Self::MergeBotAddToQueue => "Add selected PRs to the merge queue",

            // Help
            Self::KeyBindingsToggleView => "Show or hide the key bindings help panel",

            // Build Log
            Self::BuildLogOpen => "Open the build logs viewer for the current PR",
            Self::BuildLogNextError => "Jump to the next error in the build logs",
            Self::BuildLogPrevError => "Jump to the previous error in the build logs",
            Self::BuildLogToggle => "Toggle expand/collapse of the current tree node",
            Self::BuildLogToggleTimestamps => "Toggle timestamp display in log lines",
            Self::BuildLogExpandAll => "Expand all nodes in the build log tree",
            Self::BuildLogCollapseAll => "Collapse all nodes in the build log tree",

            // Diff Viewer
            Self::DiffViewerOpen => {
                "Open the diff viewer to review PR changes with syntax highlighting"
            }
            Self::DiffViewerSwitchPane => "Switch focus between file tree and diff content pane",
            Self::DiffViewerAddComment => "Add a comment on the current line or selection",
            Self::DiffViewerSubmitComment => "Submit the current comment",
            Self::DiffViewerCancelComment => "Cancel comment editing and discard changes",
            Self::DiffViewerVisualMode => "Enter visual mode for multi-line selection",
            Self::DiffViewerShowReviewPopup => {
                "Show the review submission popup (Approve, Request Changes, Comment)"
            }
            Self::DiffViewerPageDown => "Scroll down one page in the diff viewer",
            Self::DiffViewerPageUp => "Scroll up one page in the diff viewer",

            // General
            Self::GlobalClose => "Close the current view or panel",
            Self::GlobalQuit => "Exit the application",
        }
    }

    /// Get the category for this command (used for grouping in command palette)
    pub fn category(&self) -> &'static str {
        match self {
            Self::Confirm | Self::ToggleSelect | Self::SelectAll | Self::DeselectAll => "Selection",

            Self::RepositoryAdd
            | Self::RepositoryRemove
            | Self::RepositoryOpenInBrowser
            | Self::RepositoryNext
            | Self::RepositoryPrevious => "Repository",

            Self::NavigateNext
            | Self::NavigatePrevious
            | Self::NavigateLeft
            | Self::NavigateRight
            | Self::NavigateToTop
            | Self::NavigateToBottom => "Navigation",

            Self::DebugToggleConsoleView | Self::DebugClearLogs => "Debug",

            Self::CommandPaletteOpen => "Command Palette",

            Self::PrToggleSelection
            | Self::PrSelectAll
            | Self::PrDeselectAll
            | Self::PrRefresh
            | Self::PrOpenInBrowser
            | Self::PrMerge
            | Self::PrRebase
            | Self::PrApprove
            | Self::PrComment
            | Self::PrRequestChanges
            | Self::PrClose
            | Self::PrRerunFailedJobs
            | Self::PrOpenBuildLogs
            | Self::PrOpenInIDE
            | Self::PrCycleFilter
            | Self::PrClearFilter => "Pull Request",

            Self::MergeBotStart | Self::MergeBotStop | Self::MergeBotAddToQueue => "Merge Bot",

            Self::KeyBindingsToggleView => "Help",

            Self::BuildLogOpen
            | Self::BuildLogNextError
            | Self::BuildLogPrevError
            | Self::BuildLogToggle
            | Self::BuildLogToggleTimestamps
            | Self::BuildLogExpandAll
            | Self::BuildLogCollapseAll => "Build Log",

            Self::DiffViewerOpen
            | Self::DiffViewerSwitchPane
            | Self::DiffViewerAddComment
            | Self::DiffViewerSubmitComment
            | Self::DiffViewerCancelComment
            | Self::DiffViewerVisualMode
            | Self::DiffViewerShowReviewPopup
            | Self::DiffViewerPageDown
            | Self::DiffViewerPageUp => "Diff Viewer",

            Self::GlobalClose | Self::GlobalQuit => "General",
        }
    }

    pub fn category_order() -> Vec<&'static str> {
        vec![
            "Selection",
            "Navigation",
            "Repository",
            "Pull Request",
            "Build Log",
            "Diff Viewer",
            "Merge Bot",
            "Command Palette",
            "Debug",
            "Help",
            "General",
        ]
    }

    /// Check if this command should appear in the command palette
    /// there is a `palette_command_ids()` in `crates/gh-pr-lander/src/commands.rs`
    /// that always gets forgotten and needs manual maintenance,
    /// its defining which commands are in the pallette,
    /// its actaully a semantical duplication to this very function here
    pub fn show_in_palette(&self) -> bool {
        match self {
            // Semantic/context commands are keyboard-driven, not shown in palette
            Self::Confirm | Self::ToggleSelect | Self::SelectAll | Self::DeselectAll => false,

            // Navigation/scroll commands are typically not shown in palette
            // (they're keyboard-driven)
            Self::NavigateNext
            | Self::NavigatePrevious
            | Self::NavigateLeft
            | Self::NavigateRight
            | Self::NavigateToTop
            | Self::NavigateToBottom
            | Self::CommandPaletteOpen => false,

            // Build log navigation commands are keyboard-driven within the view
            Self::BuildLogNextError
            | Self::BuildLogPrevError
            | Self::BuildLogToggle
            | Self::BuildLogToggleTimestamps
            | Self::BuildLogExpandAll
            | Self::BuildLogCollapseAll => false,

            // Diff viewer view-specific commands are keyboard-driven
            Self::DiffViewerSwitchPane
            | Self::DiffViewerAddComment
            | Self::DiffViewerSubmitComment
            | Self::DiffViewerCancelComment
            | Self::DiffViewerVisualMode
            | Self::DiffViewerShowReviewPopup
            | Self::DiffViewerPageDown
            | Self::DiffViewerPageUp => false,

            // MergeBot is not yet tested nor stable
            Self::MergeBotAddToQueue | Self::MergeBotStart | Self::MergeBotStop => false,

            // All others are shown (including DiffViewerOpen)
            _ => true,
        }
    }

    /// Get all command IDs that should appear in the command palette
    pub fn palette_command_ids() -> Vec<CommandId> {
        use strum::IntoEnumIterator;

        Self::iter().filter(|id| id.show_in_palette()).collect()
    }
}
