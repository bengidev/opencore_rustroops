//! Application state held at the composition root.

use crate::shared::preferences::{AppPreferences, PreferencesError, PreferencesStore};
use crate::shared::theme::ThemeMode;
use super::boot::boot_screen;
use super::onboarding::OnboardingOutcome;

/// Top-level screen routing enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveScreen {
    Onboarding,
    Shell,
}

/// Window dimensions to apply when onboarding completes (GPUI layer applies in PRD #2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowResizeIntent {
    pub width: u32,
    pub height: u32,
}

/// Global application state: routing, theme, and preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub active_screen: ActiveScreen,
    pub theme_mode: ThemeMode,
    pub preferences: AppPreferences,
    pub pending_window_resize: Option<WindowResizeIntent>,
}

impl AppState {
    pub fn from_preferences(preferences: AppPreferences) -> Self {
        let theme_mode = preferences.theme_mode;
        let active_screen = boot_screen(&preferences);
        Self {
            active_screen,
            theme_mode,
            preferences,
            pending_window_resize: None,
        }
    }

    /// Marks onboarding complete, persists preferences, and routes to shell.
    pub fn complete_onboarding<S: PreferencesStore>(
        &mut self,
        store: &S,
    ) -> Result<(), PreferencesError> {
        self.apply_onboarding_outcome(OnboardingOutcome::Completed, store)
    }

    /// Applies a reducer outcome: persist and route when completed.
    pub fn apply_onboarding_outcome<S: PreferencesStore>(
        &mut self,
        outcome: OnboardingOutcome,
        store: &S,
    ) -> Result<(), PreferencesError> {
        match outcome {
            OnboardingOutcome::Pending => {}
            OnboardingOutcome::Completed => {
                let mut updated = self.preferences.clone();
                updated.onboarding_completed = true;
                store.save(&updated)?;
                self.preferences = updated;
                self.active_screen = ActiveScreen::Shell;
                self.pending_window_resize = Some(WindowResizeIntent {
                    width: 1280,
                    height: 800,
                });
            }
        }
        Ok(())
    }
}
