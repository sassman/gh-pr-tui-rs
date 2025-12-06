//! Application State Module
//!
//! Contains all state types used by the application, organized by feature.

mod add_repo;
mod app;
mod build_log;
mod command_palette;
mod confirmation_popup;
mod debug_console;
mod diff_viewer;
mod key_bindings;
mod main_view;
mod merge_bot;
mod splash;
mod status_bar;

pub use add_repo::{AddRepoField, AddRepoFormState};
pub use app::AppState;
pub use build_log::{
    BuildLogJobMetadata, BuildLogJobStatus, BuildLogLoadingState, BuildLogPrContext, BuildLogState,
};
pub use command_palette::CommandPaletteState;
pub use confirmation_popup::{ConfirmationIntent, ConfirmationPopupState};
pub use debug_console::DebugConsoleState;
pub use diff_viewer::DiffViewerState;
pub use key_bindings::KeyBindingsPanelState;
pub use main_view::{MainViewState, PrFilter, RepositoryData};
pub use merge_bot::MergeBotState;
pub use splash::SplashState;
pub use status_bar::{StatusBarState, StatusKind, StatusMessage};
