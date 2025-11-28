use crate::actions::Action;
use crate::state::SplashState;

/// Reducer for splash screen state
pub fn reduce(mut state: SplashState, action: &Action) -> SplashState {
    match action {
        Action::BootstrapStart => {
            state.bootstrapping = true;
            state.animation_frame = 0;
        }
        Action::BootstrapEnd => {
            state.bootstrapping = false;
        }
        Action::Tick if state.bootstrapping => {
            // Advance animation frame (16 frames total for 5x5 snake)
            state.animation_frame += 1;
        }
        _ => {
            // Unhandled actions - no state change
        }
    }

    state
}
