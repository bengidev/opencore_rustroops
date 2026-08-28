//! Welcome command reducer, interactive state, and view.

pub mod theme_toggle;
mod ui_state;
mod view;

pub use ui_state::WelcomeUiState;
pub use view::{WelcomeCallbacks, welcome_interactive_root, welcome_screen};

/// Commands the welcome UI can send to the reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeCommand {
    EnterPressed,
}

/// Outcomes produced by the welcome reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeOutcome {
    /// No state change; welcome continues.
    Pending,
    /// User finished welcome (primary CTA).
    Completed,
}

/// Reduces a welcome command to an outcome.
pub fn reduce_welcome(command: WelcomeCommand) -> WelcomeOutcome {
    match command {
        WelcomeCommand::EnterPressed => WelcomeOutcome::Completed,
    }
}
