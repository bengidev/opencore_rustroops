//! Application state held at the composition root.

use super::app_boot::boot_screen;
use super::onboarding::OnboardingOutcome;
use crate::shared::preferences::{AppPreferences, PreferencesError, PreferencesStore};
use crate::shared::theme::ThemeMode;

/// Onboarding window width (reference layout proportions).
pub const ONBOARDING_WINDOW_WIDTH: u32 = 960;

/// Onboarding window height.
pub const ONBOARDING_WINDOW_HEIGHT: u32 = 680;

/// Shell window width after onboarding (960×680 → 1280×800).
pub const SHELL_WINDOW_WIDTH: u32 = 1280;

/// Shell window height after onboarding.
pub const SHELL_WINDOW_HEIGHT: u32 = 800;

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

/// Global application state: routing and preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub active_screen: ActiveScreen,
    pub preferences: AppPreferences,
    pub pending_window_resize: Option<WindowResizeIntent>,
}

impl AppState {
    pub fn from_preferences(preferences: AppPreferences) -> Self {
        let active_screen = boot_screen(&preferences);
        Self {
            active_screen,
            preferences,
            pending_window_resize: None,
        }
    }

    /// Active theme from persisted preferences (single source of truth).
    pub fn theme_mode(&self) -> ThemeMode {
        self.preferences.theme_mode
    }

    /// Initial window dimensions for the active screen at launch.
    pub fn initial_window_size(&self) -> (u32, u32) {
        match self.active_screen {
            ActiveScreen::Onboarding => (ONBOARDING_WINDOW_WIDTH, ONBOARDING_WINDOW_HEIGHT),
            ActiveScreen::Shell => (SHELL_WINDOW_WIDTH, SHELL_WINDOW_HEIGHT),
        }
    }

    /// Consumes a pending resize intent after the GPU layer applies it.
    pub fn take_pending_window_resize(&mut self) -> Option<WindowResizeIntent> {
        self.pending_window_resize.take()
    }

    /// Marks onboarding complete, persists preferences, and routes to shell.
    pub fn complete_onboarding<S: PreferencesStore>(
        &mut self,
        store: &S,
    ) -> Result<(), PreferencesError> {
        self.apply_onboarding_outcome(OnboardingOutcome::Completed, store)
    }

    /// Persists a theme change from onboarding controls.
    pub fn set_theme_mode<S: PreferencesStore>(
        &mut self,
        store: &S,
        mode: ThemeMode,
    ) -> Result<(), PreferencesError> {
        let mut updated = self.preferences.clone();
        updated.theme_mode = mode;
        store.save(&updated)?;
        self.preferences = updated;
        Ok(())
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
                    width: SHELL_WINDOW_WIDTH,
                    height: SHELL_WINDOW_HEIGHT,
                });
            }
        }
        Ok(())
    }

    /// Resets persisted preferences to defaults and routes back to onboarding (dev tooling).
    pub fn reset_persistent_data<S: PreferencesStore>(
        &mut self,
        store: &S,
    ) -> Result<(), PreferencesError> {
        let defaults = AppPreferences::default();
        store.save(&defaults)?;
        self.preferences = defaults;
        self.active_screen = ActiveScreen::Onboarding;
        self.pending_window_resize = Some(WindowResizeIntent {
            width: ONBOARDING_WINDOW_WIDTH,
            height: ONBOARDING_WINDOW_HEIGHT,
        });
        Ok(())
    }
}
