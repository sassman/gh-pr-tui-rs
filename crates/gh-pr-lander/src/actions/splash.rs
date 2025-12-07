//! Splash screen actions
//!
//! Actions specific to the splash/loading screen.

/// Actions for the Splash screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplashAction {
    /// Animation frame advance
    Tick,
}
