//! SplashScreenMiddleware - manages splash screen lifecycle and spinner animation
//!
//! This middleware handles:
//! - Starting spinner animation during bootstrap
//! - Dispatching TickSpinner actions while splash screen is visible
//! - Stopping spinner animation when bootstrap completes
//! - Updating splash screen view model

use super::{BoxFuture, Dispatcher, Middleware};
use crate::{
    actions::Action,
    state::{AppState, BootstrapState},
};
use std::time::{Duration, Instant};

/// Middleware for managing splash screen and spinner animation
pub struct SplashScreenMiddleware {
    /// Timestamp of last spinner tick (to throttle to 100ms intervals)
    last_tick: Option<Instant>,
    /// Whether splash screen is currently active
    splash_active: bool,
}

impl SplashScreenMiddleware {
    pub fn new() -> Self {
        Self {
            last_tick: None,
            splash_active: true, // Start active (bootstrap begins immediately)
        }
    }

    /// Check if splash screen should be visible based on bootstrap state
    fn is_splash_visible(bootstrap_state: &BootstrapState) -> bool {
        !matches!(
            bootstrap_state,
            BootstrapState::UIReady
                | BootstrapState::LoadingRemainingRepos
                | BootstrapState::Completed
        )
    }

    /// Check if enough time has passed since last tick (100ms throttle)
    fn should_tick(&mut self) -> bool {
        match self.last_tick {
            None => {
                self.last_tick = Some(Instant::now());
                true
            }
            Some(last) => {
                if last.elapsed() >= Duration::from_millis(100) {
                    self.last_tick = Some(Instant::now());
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl Default for SplashScreenMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for SplashScreenMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            // Update splash_active flag based on bootstrap state
            let should_be_active = Self::is_splash_visible(&state.infra.bootstrap_state);

            // Detect transition from active to inactive (bootstrap completed)
            if self.splash_active && !should_be_active {
                log::debug!("SplashScreenMiddleware: Bootstrap completed, stopping spinner");
                self.splash_active = false;
            }

            // If splash is active and enough time has passed, dispatch TickSpinner
            // Check on every action to ensure smooth animation
            if self.splash_active && self.should_tick() {
                dispatcher.dispatch(Action::TickSpinner);
            }

            // Update splash screen view model when bootstrap state changes
            match action {
                Action::Bootstrap
                | Action::SetBootstrapState(_)
                | Action::BootstrapComplete(_)
                | Action::TickSpinner => {
                    // View model is updated by reducer, we just need to let the action through
                }
                _ => {}
            }

            // Always continue to next middleware
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_splash_screen_lifecycle() {
        let mut middleware = SplashScreenMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        // Initial state - bootstrap not started
        let mut state = AppState::default();
        state.infra.bootstrap_state = BootstrapState::NotStarted;

        // Handle any action - should dispatch TickSpinner since splash is active
        middleware
            .handle(&Action::Bootstrap, &state, &dispatcher)
            .await;

        // Wait a bit for tick throttle
        tokio::time::sleep(Duration::from_millis(110)).await;

        // Should have dispatched TickSpinner
        let maybe_action = rx.try_recv();
        assert!(maybe_action.is_ok());
        assert!(matches!(maybe_action.unwrap(), Action::TickSpinner));

        // Transition to UIReady (bootstrap complete)
        state.infra.bootstrap_state = BootstrapState::UIReady;
        middleware
            .handle(
                &Action::SetBootstrapState(BootstrapState::UIReady),
                &state,
                &dispatcher,
            )
            .await;

        // Wait for next tick interval
        tokio::time::sleep(Duration::from_millis(110)).await;

        // Handle another action - should NOT dispatch TickSpinner (splash inactive)
        middleware.handle(&Action::Quit, &state, &dispatcher).await;

        // Should not have dispatched anything
        let maybe_action = rx.try_recv();
        assert!(maybe_action.is_err()); // Channel empty
    }

    #[tokio::test]
    async fn test_tick_throttling() {
        let mut middleware = SplashScreenMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        let state = AppState::default();

        // Dispatch multiple actions quickly - should only tick once due to throttle
        for _ in 0..5 {
            middleware
                .handle(&Action::Bootstrap, &state, &dispatcher)
                .await;
        }

        // Should have only one TickSpinner
        let first = rx.try_recv();
        assert!(first.is_ok());
        assert!(matches!(first.unwrap(), Action::TickSpinner));

        let second = rx.try_recv();
        assert!(second.is_err()); // No more ticks
    }
}
