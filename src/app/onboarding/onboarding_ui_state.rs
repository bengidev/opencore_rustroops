//! Interactive onboarding UI state (animation, orb hold).

use std::time::Instant;

use gpui::{App, FocusHandle, Window};

use super::onboarding_dynamics::dynamics_for_progress;

/// Local onboarding animation state (GPU-free).
#[derive(Debug, Clone)]
pub struct OnboardingUiState {
    pub started_at: Instant,
    pub now: Instant,
    pub is_holding: bool,
    pub hold_progress: f32,
    pub displayed_speed: f32,
    pub displayed_zoom: f32,
    focus_claimed: bool,
}

impl OnboardingUiState {
    pub fn new() -> Self {
        let now = Instant::now();
        let (initial_speed, initial_zoom) = dynamics_for_progress(0.0);
        Self {
            started_at: now,
            now,
            is_holding: false,
            hold_progress: 0.0,
            displayed_speed: initial_speed,
            displayed_zoom: initial_zoom,
            focus_claimed: false,
        }
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

    pub fn tick(&mut self, now: Instant) {
        let dt = now.saturating_duration_since(self.now).as_secs_f32();
        self.now = now;
        self.advance_orb_progress(dt);
    }

    pub fn orb_pressed(&mut self) {
        self.is_holding = true;
    }

    pub fn orb_released(&mut self) {
        self.is_holding = false;
    }

    fn advance_orb_progress(&mut self, dt: f32) {
        const HOLD_RAMP_PER_SEC: f32 = 0.6;
        const RELEASE_RAMP_PER_SEC: f32 = 0.9;

        let dt = dt.clamp(0.0, 0.25);
        let delta = if self.is_holding {
            HOLD_RAMP_PER_SEC * dt
        } else {
            -RELEASE_RAMP_PER_SEC * dt
        };
        self.hold_progress = (self.hold_progress + delta).clamp(0.0, 1.0);

        let (speed, zoom) = dynamics_for_progress(self.hold_progress);
        self.displayed_speed = speed;
        self.displayed_zoom = zoom;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_progress_increases_while_holding() {
        let mut state = OnboardingUiState::new();
        state.orb_pressed();
        let now = state.now + std::time::Duration::from_millis(200);
        state.tick(now);
        assert!(state.hold_progress > 0.0);
    }
}
