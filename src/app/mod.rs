//! Application composition root.
//!
//! Owns global [`ThemeMode`], boot routing, and onboarding completion. Uses **Facade**
//! to coordinate [`crate::shared`] modules without exposing persistence details to callers.

mod boot;
mod onboarding;
mod state;

pub use boot::boot_screen;
pub use onboarding::{OnboardingCommand, OnboardingOutcome, OnboardingState};
pub use state::{ActiveScreen, AppState, WindowResizeIntent};
