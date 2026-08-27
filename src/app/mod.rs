//! Application composition root (**Facade**): boot routing, welcome completion,
//! preferences I/O, and desktop window lifecycle.

mod boot;
mod desktop;
mod state;
#[cfg(debug_assertions)]
mod dev_reset;
mod gpui_callbacks;
pub mod shell;
mod viewport;
mod welcome;
mod window_placement;

pub use boot::boot_screen;
pub use state::{
    ActiveScreen, AppState, HOME_WINDOW_HEIGHT, HOME_WINDOW_WIDTH, WELCOME_WINDOW_HEIGHT,
    WELCOME_WINDOW_WIDTH, WindowResizeIntent,
};
pub use welcome::{WelcomeCommand, WelcomeOutcome, reduce_welcome};

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

/// Boots state from preferences without opening a window (for tests and embedders).
pub fn boot() -> Result<RunningApp, AppError> {
    let store = FilePreferencesStore::open()?;
    let preferences = store.load()?;
    let state = AppState::from_preferences(preferences);
    Ok(RunningApp { state, store })
}

/// Boots the application and runs the desktop window until it closes.
pub fn run() -> Result<(), AppError> {
    desktop::run_desktop()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::preferences::{AppPreferences, InMemoryPreferencesStore};
    use crate::shared::theme::ThemeMode;

    #[test]
    fn boot_screen_shows_onboarding_when_incomplete() {
        let prefs = AppPreferences::default();
        assert_eq!(boot_screen(&prefs), ActiveScreen::Welcome);
    }

    #[test]
    fn boot_screen_shows_home_when_onboarding_complete() {
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Dark,
            onboarding_completed: true,
            ..Default::default()
        };
        assert_eq!(boot_screen(&prefs), ActiveScreen::Home);
    }

    #[test]
    fn boot_screen_ignores_theme_mode_for_routing() {
        for theme in [ThemeMode::Light, ThemeMode::Dark] {
            let incomplete = AppPreferences {
                theme_mode: theme,
                onboarding_completed: false,
                ..Default::default()
            };
            assert_eq!(boot_screen(&incomplete), ActiveScreen::Welcome);

            let complete = AppPreferences {
                theme_mode: theme,
                onboarding_completed: true,
                ..Default::default()
            };
            assert_eq!(boot_screen(&complete), ActiveScreen::Home);
        }
    }

    #[test]
    fn app_state_restores_theme_from_preferences() {
        let prefs = AppPreferences {
            theme_mode: ThemeMode::Light,
            onboarding_completed: false,
            ..Default::default()
        };
        let state = AppState::from_preferences(prefs);
        assert_eq!(state.theme_mode(), ThemeMode::Light);
        assert_eq!(state.active_screen, ActiveScreen::Welcome);
    }

    #[test]
    fn completing_onboarding_persists_and_routes_to_home() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        state.complete_welcome(&store).expect("complete welcome");

        assert!(state.preferences.onboarding_completed);
        assert_eq!(state.active_screen, ActiveScreen::Home);
        let loaded = store.load().expect("load");
        assert!(loaded.onboarding_completed);
    }

    #[test]
    fn completing_onboarding_records_window_resize_intent() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        state.complete_welcome(&store).expect("complete welcome");

        let intent = state.pending_window_resize.expect("resize intent recorded");
        assert_eq!(intent.width, HOME_WINDOW_WIDTH);
        assert_eq!(intent.height, HOME_WINDOW_HEIGHT);
    }

    #[test]
    fn initial_window_size_matches_active_screen() {
        let incomplete = AppState::from_preferences(AppPreferences::default());
        assert_eq!(
            incomplete.initial_window_size(),
            (WELCOME_WINDOW_WIDTH, WELCOME_WINDOW_HEIGHT)
        );

        let complete = AppState::from_preferences(AppPreferences {
            theme_mode: ThemeMode::Dark,
            onboarding_completed: true,
            ..Default::default()
        });
        assert_eq!(
            complete.initial_window_size(),
            (HOME_WINDOW_WIDTH, HOME_WINDOW_HEIGHT)
        );
    }

    #[test]
    fn onboarding_enter_yields_completed_outcome() {
        assert_eq!(
            reduce_welcome(WelcomeCommand::EnterPressed),
            WelcomeOutcome::Completed
        );
    }

    #[test]
    fn reset_persistent_data_routes_to_onboarding() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences {
            theme_mode: ThemeMode::Light,
            onboarding_completed: true,
            ..Default::default()
        });
        state
            .reset_persistent_data(&store)
            .expect("reset persistent data");

        assert_eq!(state.active_screen, ActiveScreen::Welcome);
        assert!(!state.preferences.onboarding_completed);
        let loaded = store.load().expect("load");
        assert_eq!(loaded, AppPreferences::default());
        let intent = state
            .pending_window_resize
            .expect("onboarding resize intent");
        assert_eq!(intent.width, WELCOME_WINDOW_WIDTH);
        assert_eq!(intent.height, WELCOME_WINDOW_HEIGHT);
    }

    #[test]
    fn app_handles_onboarding_completion_via_store() {
        let store = InMemoryPreferencesStore::new();
        let mut state = AppState::from_preferences(AppPreferences::default());
        let outcome = reduce_welcome(WelcomeCommand::EnterPressed);
        assert_eq!(outcome, WelcomeOutcome::Completed);
        state
            .apply_welcome_outcome(outcome, &store)
            .expect("apply outcome");

        assert_eq!(state.active_screen, ActiveScreen::Home);
        let saved = store.load().expect("load");
        assert!(saved.onboarding_completed);
    }
}
