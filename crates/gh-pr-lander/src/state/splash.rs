//! Splash Screen State

use std::time::Instant;

/// Minimum time the splash screen should be visible (in seconds)
pub const MIN_SPLASH_DURATION_SECS: f64 = 2.0;

/// Splash screen state
#[derive(Debug, Clone)]
pub struct SplashState {
    pub bootstrapping: bool,
    pub animation_frame: usize, // Current frame of the snake animation (0-15)
    /// When the splash screen started (for minimum display time)
    pub started_at: Option<Instant>,
    /// Whether bootstrap has finished loading (but we might still show splash)
    pub loading_complete: bool,
}

impl Default for SplashState {
    fn default() -> Self {
        Self {
            bootstrapping: true,
            animation_frame: 0,
            started_at: Some(Instant::now()),
            loading_complete: false,
        }
    }
}

impl SplashState {
    /// Check if the minimum splash duration has elapsed
    pub fn min_duration_elapsed(&self) -> bool {
        self.started_at
            .map(|start| start.elapsed().as_secs_f64() >= MIN_SPLASH_DURATION_SECS)
            .unwrap_or(true)
    }

    /// Check if splash can be dismissed (loading complete AND min duration elapsed)
    pub fn can_dismiss(&self) -> bool {
        self.loading_complete && self.min_duration_elapsed()
    }
}
