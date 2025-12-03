//! Application State Module
//!
//! Contains all state types used by the application, organized by feature.

mod add_repo;
mod app;
mod command_palette;
mod debug_console;
mod key_bindings;
mod main_view;
mod merge_bot;
mod splash;

pub use add_repo::{AddRepoField, AddRepoFormState};
pub use app::AppState;
pub use command_palette::CommandPaletteState;
pub use debug_console::DebugConsoleState;
pub use key_bindings::KeyBindingsPanelState;
pub use main_view::{MainViewState, PrFilter, RepositoryData};
pub use merge_bot::MergeBotState;
pub use splash::SplashState;
