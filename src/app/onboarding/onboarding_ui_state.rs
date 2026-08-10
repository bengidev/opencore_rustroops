//! Interactive onboarding UI state (keyboard focus + ASCII hero animation).

use std::time::Instant;

use gpui::{App, FocusHandle, Window};

use super::ascii_galaxy::{DEFAULT_SEED, GalaxyAscii};

pub struct OnboardingUiState {
    galaxy: GalaxyAscii,
    last_tick: Instant,
    focus_claimed: bool,
}

impl OnboardingUiState {
    pub fn new() -> Self {
        let mut galaxy = GalaxyAscii::new(DEFAULT_SEED);
        let _ = galaxy.tick(0.0);
        Self {
            galaxy,
            last_tick: Instant::now(),
            focus_claimed: false,
        }
    }

    pub fn tick(&mut self, now: Instant) {
        let dt = now
            .saturating_duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(0.0, 0.1);
        self.last_tick = now;
        let _ = self.galaxy.tick(dt);
    }

    pub fn last_frame(&self) -> &str {
        self.galaxy.frame()
    }

    /// Requests keyboard focus once per onboarding session.
    pub fn ensure_initial_focus(
        &mut self,
        window: &mut Window,
        handle: &FocusHandle,
        cx: &mut App,
    ) {
        if self.focus_claimed {
            return;
        }
        if handle.is_focused(window) {
            self.focus_claimed = true;
        } else {
            window.focus(handle, cx);
        }
    }
}
