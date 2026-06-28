//! Onboarding state machine (logic only — no view).

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

/// Onboarding reducer using **Command** messages and **State** outcomes.
pub struct OnboardingState;

impl OnboardingState {
    pub fn reduce(command: OnboardingCommand) -> OnboardingOutcome {
        match command {
            OnboardingCommand::EnterPressed => OnboardingOutcome::Completed,
        }
    }
}
