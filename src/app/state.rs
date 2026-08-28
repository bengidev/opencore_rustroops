//! Application state held at the composition root.

use super::boot::boot_screen;
use super::welcome::WelcomeOutcome;
use crate::shared::preferences::{AppPreferences, PreferencesError, PreferencesStore};
use crate::shared::theme::ThemeMode;

/// Welcome window width (960×740 landing layout).
pub const WELCOME_WINDOW_WIDTH: u32 = 960;

/// Welcome window height.
pub const WELCOME_WINDOW_HEIGHT: u32 = 740;

/// Home window width after welcome (960×740 → 1280×800).
pub const HOME_WINDOW_WIDTH: u32 = 1280;

/// Home window height after welcome.
pub const HOME_WINDOW_HEIGHT: u32 = 800;

/// Top-level screen routing enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveScreen {
    Welcome,
    Home,
}

/// Window dimensions to apply when welcome completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowResizeIntent {
    pub width: u32,
    pub height: u32,
}

/// Global application state: routing and preferences.
#[derive(Debug, Clone, PartialEq)]
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
            ActiveScreen::Welcome => (WELCOME_WINDOW_WIDTH, WELCOME_WINDOW_HEIGHT),
            ActiveScreen::Home => (HOME_WINDOW_WIDTH, HOME_WINDOW_HEIGHT),
        }
    }

    /// Consumes a pending resize intent after the GPU layer applies it.
    pub fn take_pending_window_resize(&mut self) -> Option<WindowResizeIntent> {
        self.pending_window_resize.take()
    }

    /// Marks welcome complete, persists preferences, and routes to home.
    pub fn complete_welcome<S: PreferencesStore>(
        &mut self,
        store: &S,
    ) -> Result<(), PreferencesError> {
        self.apply_welcome_outcome(WelcomeOutcome::Completed, store)
    }

    /// Persists a theme change from welcome controls.
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

    /// Persists onboarding completion and queues the home resize without routing.
    pub fn persist_welcome_completion<S: PreferencesStore>(
        &mut self,
        store: &S,
    ) -> Result<(), PreferencesError> {
        let mut updated = self.preferences.clone();
        updated.onboarding_completed = true;
        store.save(&updated)?;
        self.preferences = updated;
        self.pending_window_resize = Some(WindowResizeIntent {
            width: HOME_WINDOW_WIDTH,
            height: HOME_WINDOW_HEIGHT,
        });
        Ok(())
    }

    /// Routes to home after the welcome hero transition finishes.
    pub fn finish_welcome_transition(&mut self) {
        self.active_screen = ActiveScreen::Home;
    }

    /// Applies a reducer outcome: persist and route when completed.
    pub fn apply_welcome_outcome<S: PreferencesStore>(
        &mut self,
        outcome: WelcomeOutcome,
        store: &S,
    ) -> Result<(), PreferencesError> {
        match outcome {
            WelcomeOutcome::Pending => {}
            WelcomeOutcome::Completed => {
                self.persist_welcome_completion(store)?;
                self.finish_welcome_transition();
            }
        }
        Ok(())
    }

    /// Resets persisted preferences to defaults and routes back to welcome (dev tooling).
    pub fn reset_persistent_data<S: PreferencesStore>(
        &mut self,
        store: &S,
    ) -> Result<(), PreferencesError> {
        let defaults = AppPreferences::default();
        store.save(&defaults)?;
        self.preferences = defaults;
        self.active_screen = ActiveScreen::Welcome;
        self.pending_window_resize = Some(WindowResizeIntent {
            width: WELCOME_WINDOW_WIDTH,
            height: WELCOME_WINDOW_HEIGHT,
        });
        Ok(())
    }
}
