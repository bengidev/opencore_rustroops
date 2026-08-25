//! Boot-time screen selection from persisted preferences.

use super::ActiveScreen;
use crate::shared::preferences::AppPreferences;

/// Selects the initial screen from persisted welcome completion.
pub fn boot_screen(preferences: &AppPreferences) -> ActiveScreen {
    if preferences.onboarding_completed {
        ActiveScreen::Home
    } else {
        ActiveScreen::Welcome
    }
}
