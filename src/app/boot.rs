//! Boot-time screen selection from persisted preferences.

use crate::shared::preferences::AppPreferences;
use super::ActiveScreen;

/// Selects the initial screen based on onboarding completion.
pub fn boot_screen(preferences: &AppPreferences) -> ActiveScreen {
    if preferences.onboarding_completed {
        ActiveScreen::Shell
    } else {
        ActiveScreen::Onboarding
    }
}
