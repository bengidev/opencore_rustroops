//! Onboarding command reducer (logic only — no view).
//!
//! Commands map to outcomes; [`crate::app::AppState::apply_onboarding_outcome`] handles
//! persistence and routing when an outcome is [`OnboardingOutcome::Completed`].

/// Commands the onboarding UI can send to the reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingCommand {
    EnterPressed,
}

/// Outcomes produced by the onboarding reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingOutcome {
    /// No state change; onboarding continues.
    Pending,
    /// User finished onboarding (ENTER pressed).
    Completed,
}

/// Reduces an onboarding command to an outcome.
pub fn reduce_onboarding(command: OnboardingCommand) -> OnboardingOutcome {
    match command {
        OnboardingCommand::EnterPressed => OnboardingOutcome::Completed,
    }
}
