//! MergeBotMiddleware - manages merge bot lifecycle and periodic ticking
//!
//! This middleware handles:
//! - Starting merge bot tick timer when bot starts
//! - Dispatching MergeBotTick actions while bot is running
//! - Stopping tick timer when bot completes or stops
//! - Managing bot state transitions

use super::{BoxFuture, Dispatcher, Middleware};
use crate::{actions::Action, state::AppState};
use std::time::{Duration, Instant};

/// Middleware for managing merge bot lifecycle and periodic ticking
pub struct MergeBotMiddleware {
    /// Timestamp of last bot tick (to throttle to 100ms intervals)
    last_tick: Option<Instant>,
    /// Whether bot was running in previous check (to detect state transitions)
    was_running: bool,
}

impl MergeBotMiddleware {
    pub fn new() -> Self {
        Self {
            last_tick: None,
            was_running: false,
        }
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

impl Default for MergeBotMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for MergeBotMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            let is_running = state.merge_bot.bot.is_running();

            // Detect transition from not running to running (bot started)
            if !self.was_running && is_running {
                log::debug!("MergeBotMiddleware: Merge bot started, beginning periodic ticks");
                self.was_running = true;
                // Reset tick timer so we can tick immediately
                self.last_tick = None;
            }

            // Detect transition from running to not running (bot stopped)
            if self.was_running && !is_running {
                log::debug!("MergeBotMiddleware: Merge bot stopped, ending periodic ticks");
                self.was_running = false;
                self.last_tick = None;
            }

            // If bot is running and enough time has passed, dispatch MergeBotTick
            // Check on every action to ensure smooth bot operation
            if is_running && self.should_tick() {
                dispatcher.dispatch(Action::MergeBotTick);
            }

            // Actions that might affect bot state (for logging/debugging)
            match action {
                Action::StartMergeBot | Action::StartMergeBotWithPrData(_) => {
                    // Bot starting - reducer will update state, we'll detect it on next action
                }
                Action::MergeBotTick => {
                    // Bot tick being processed
                }
                Action::MergeComplete(_) => {
                    // Bot might be completing
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
    use crate::merge_bot::MergeBot;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_merge_bot_lifecycle() {
        let mut middleware = MergeBotMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        // Initial state - bot not running
        let mut state = AppState::default();
        assert!(!state.merge_bot.bot.is_running());

        // Handle action while bot is not running - should not dispatch MergeBotTick
        middleware
            .handle(&Action::Bootstrap, &state, &dispatcher)
            .await;

        let maybe_action = rx.try_recv();
        assert!(maybe_action.is_err()); // No tick dispatched

        // Simulate bot starting (create a bot and start it with PRs)
        state.merge_bot.bot = MergeBot::new();
        state.merge_bot.bot.start(vec![(123, 0)]);
        assert!(state.merge_bot.bot.is_running());

        // Wait for tick throttle
        tokio::time::sleep(Duration::from_millis(110)).await;

        // Handle action while bot is running - should dispatch MergeBotTick
        middleware
            .handle(&Action::Bootstrap, &state, &dispatcher)
            .await;

        let maybe_action = rx.try_recv();
        assert!(maybe_action.is_ok());
        assert!(matches!(maybe_action.unwrap(), Action::MergeBotTick));

        // Simulate bot stopping (no more PRs)
        state.merge_bot.bot = MergeBot::default();
        assert!(!state.merge_bot.bot.is_running());

        // Wait for next tick interval
        tokio::time::sleep(Duration::from_millis(110)).await;

        // Handle action after bot stopped - should not dispatch MergeBotTick
        middleware
            .handle(&Action::Bootstrap, &state, &dispatcher)
            .await;

        let maybe_action = rx.try_recv();
        assert!(maybe_action.is_err()); // No more ticks
    }

    #[tokio::test]
    async fn test_tick_throttling() {
        let mut middleware = MergeBotMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        // Bot running
        let mut state = AppState::default();
        state.merge_bot.bot = MergeBot::new();
        state.merge_bot.bot.start(vec![(123, 0)]);

        // Dispatch multiple actions quickly - should only tick once due to throttle
        for _ in 0..5 {
            middleware
                .handle(&Action::Bootstrap, &state, &dispatcher)
                .await;
        }

        // Should have only one MergeBotTick
        let first = rx.try_recv();
        assert!(first.is_ok());
        assert!(matches!(first.unwrap(), Action::MergeBotTick));

        let second = rx.try_recv();
        assert!(second.is_err()); // No more ticks
    }

    #[tokio::test]
    async fn test_no_ticks_when_not_running() {
        let mut middleware = MergeBotMiddleware::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        // Bot not running
        let state = AppState::default();
        assert!(!state.merge_bot.bot.is_running());

        // Wait for tick interval
        tokio::time::sleep(Duration::from_millis(110)).await;

        // Handle many actions - should never dispatch MergeBotTick
        for _ in 0..10 {
            middleware
                .handle(&Action::Bootstrap, &state, &dispatcher)
                .await;
        }

        // Should have no ticks
        let maybe_action = rx.try_recv();
        assert!(maybe_action.is_err());
    }
}
