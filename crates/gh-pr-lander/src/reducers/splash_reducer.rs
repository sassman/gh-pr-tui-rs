use crate::actions::SplashAction;
use crate::state::SplashState;

/// Reducer for splash screen state.
///
/// Accepts only SplashAction, making it type-safe and focused.
pub fn reduce_splash(mut state: SplashState, action: &SplashAction) -> SplashState {
    match action {
        SplashAction::Tick => {
            if state.bootstrapping {
                // Advance animation frame (16 frames total for 5x5 snake)
                state.animation_frame += 1;
            }
        }
    }
    state
}
