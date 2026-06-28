//! Application composition root: boot routing, onboarding completion, and preferences I/O.

mod boot;
mod onboarding;
mod state;

pub use boot::boot_screen;
pub use onboarding::{reduce_onboarding, OnboardingCommand, OnboardingOutcome};
pub use state::{
    ActiveScreen, AppState, WindowResizeIntent, SHELL_WINDOW_HEIGHT, SHELL_WINDOW_WIDTH,
};

use crate::shared::preferences::{FilePreferencesStore, PreferencesError, PreferencesStore};
use thiserror::Error;

/// Errors surfaced by the application entry point.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("preferences error: {0}")]
    Preferences(#[from] PreferencesError),
}

/// Booted application: composed state and the preferences store that loaded it.
pub struct RunningApp {
    pub state: AppState,
    pub store: FilePreferencesStore,
}

/// Boots the application: load preferences, restore theme, select initial screen.
///
/// GPU window wiring is deferred to a later PRD; this entry initializes state for callers.
pub fn run() -> Result<RunningApp, AppError> {
    let store = FilePreferencesStore::open()?;
    let preferences = store.load()?;
    let state = AppState::from_preferences(preferences);
    Ok(RunningApp { state, store })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::preferences::{AppPreferences, InMemoryPreferencesStore};
    use crate::shared::theme::ThemeMode;

    #[test]
    fn boot_screen_shows_onboarding_when_incomplete() {
        let prefs = AppPreferences::default();
        assert_eq!(boot_screen(&prefs), ActiveScreen::Onboarding);
    }

    #[test]
    fn boot_screen_shows_shell_when_onboarding_complete() {
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Dark,
            onboarding_completed: true,
        };
        assert_eq!(boot_screen(&prefs), ActiveScreen::Shell);
    }

    #[test]
    fn boot_screen_ignores_theme_mode_for_routing() {
        for theme in [ThemeMode::Light, ThemeMode::Dark] {
            let incomplete = AppPreferences {
                theme_mode: theme,
                onboarding_completed: false,
            };
            assert_eq!(boot_screen(&incomplete), ActiveScreen::Onboarding);

            let complete = AppPreferences {
                theme_mode: theme,
                onboarding_completed: true,
            };
            assert_eq!(boot_screen(&complete), ActiveScreen::Shell);
        }
    }

    #[test]
    fn app_state_restores_theme_from_preferences() {
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Light,
            onboarding_completed: false,
        };
        let state = AppState::from_preferences(prefs);
        assert_eq!(state.theme_mode(), ThemeMode::Light);
        assert_eq!(state.active_screen, ActiveScreen::Onboarding);
    }

    #[test]
    fn completing_onboarding_persists_and_routes_to_shell() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        state
            .complete_onboarding(&store)
            .expect("complete onboarding");

        assert!(state.preferences.onboarding_completed);
        assert_eq!(state.active_screen, ActiveScreen::Shell);
        let loaded = store.load().expect("load");
        assert!(loaded.onboarding_completed);
    }

    #[test]
    fn completing_onboarding_records_window_resize_intent() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        state
            .complete_onboarding(&store)
            .expect("complete onboarding");

        let intent = state
            .pending_window_resize
            .expect("resize intent recorded");
        assert_eq!(intent.width, SHELL_WINDOW_WIDTH);
        assert_eq!(intent.height, SHELL_WINDOW_HEIGHT);
    }

    #[test]
    fn onboarding_enter_yields_completed_outcome() {
        assert_eq!(
            reduce_onboarding(OnboardingCommand::EnterPressed),
            OnboardingOutcome::Completed
        );
    }

    #[test]
    fn app_handles_onboarding_completion_via_store() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        let outcome = reduce_onboarding(OnboardingCommand::EnterPressed);
        assert_eq!(outcome, OnboardingOutcome::Completed);
        state
            .apply_onboarding_outcome(outcome, &store)
            .expect("apply outcome");

        assert_eq!(state.active_screen, ActiveScreen::Shell);
        let saved = store.load().expect("load");
        assert!(saved.onboarding_completed);
    }
}
