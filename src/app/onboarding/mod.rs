//! Onboarding command reducer, interactive state, and view.

mod onboarding_draw;
mod onboarding_dynamics;
mod onboarding_galaxy_orb;
mod onboarding_scene_backdrop;
mod onboarding_theme_toggle;
mod onboarding_ui_state;
mod onboarding_view;

pub use onboarding_ui_state::OnboardingUiState;
pub use onboarding_view::{OnboardingCallbacks, onboarding_screen};

/// Commands the onboarding UI can send to the reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingCommand {
    EnterPressed,
    Skipped,
}

/// Outcomes produced by the onboarding reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingOutcome {
    /// No state change; onboarding continues.
    Pending,
    /// User finished onboarding (primary CTA).
    Completed,
    /// User skipped onboarding.
    Skipped,
}

/// Reduces an onboarding command to an outcome.
pub fn reduce_onboarding(command: OnboardingCommand) -> OnboardingOutcome {
    match command {
        OnboardingCommand::EnterPressed => OnboardingOutcome::Completed,
        OnboardingCommand::Skipped => OnboardingOutcome::Skipped,
    }
}

#[cfg(test)]
mod reducer_tests {
    use super::*;

    #[test]
    fn skip_yields_skipped_outcome() {
        assert_eq!(
            reduce_onboarding(OnboardingCommand::Skipped),
            OnboardingOutcome::Skipped
        );
    }
}
