//! Interactive onboarding UI state (animation, selection, orb hold).

use std::time::Instant;

use super::onboarding_dynamics::dynamics_for_progress;
use super::onboarding_feature_card_dynamics::{approach, highlight_target};

pub const FEATURE_COUNT: usize = 4;

/// Local onboarding animation and selection state (GPU-free).
#[derive(Debug, Clone)]
pub struct OnboardingUiState {
    pub started_at: Instant,
    pub now: Instant,
    pub selected_feature: usize,
    pub hovered_feature: Option<usize>,
    pub feature_glow: [f32; FEATURE_COUNT],
    pub is_holding: bool,
    pub hold_progress: f32,
    pub displayed_speed: f32,
    pub displayed_zoom: f32,
}

impl OnboardingUiState {
    pub fn new() -> Self {
        let now = Instant::now();
        let (initial_speed, initial_zoom) = dynamics_for_progress(0.0);
        Self {
            started_at: now,
            now,
            selected_feature: 0,
            hovered_feature: None,
            feature_glow: [0.0; FEATURE_COUNT],
            is_holding: false,
            hold_progress: 0.0,
            displayed_speed: initial_speed,
            displayed_zoom: initial_zoom,
        }
    }

    pub fn tick(&mut self, now: Instant) {
        let dt = now.saturating_duration_since(self.now).as_secs_f32();
        self.now = now;
        self.advance_orb_progress(dt);
        self.advance_feature_glow(dt);
    }

    pub fn orb_pressed(&mut self) {
        self.is_holding = true;
    }

    pub fn orb_released(&mut self) {
        self.is_holding = false;
    }

    pub fn select_feature(&mut self, index: usize) {
        if index < FEATURE_COUNT {
            self.selected_feature = index;
        }
    }

    pub fn hover_feature(&mut self, index: Option<usize>) {
        self.hovered_feature = index.filter(|i| *i < FEATURE_COUNT);
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

    fn advance_feature_glow(&mut self, dt: f32) {
        for index in 0..FEATURE_COUNT {
            let hovered = self.hovered_feature == Some(index);
            let selected = self.selected_feature == index;
            let target = highlight_target(selected, hovered);
            self.feature_glow[index] = approach(self.feature_glow[index], target, dt, 9.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_selection_clamps_to_valid_range() {
        let mut state = OnboardingUiState::new();
        state.select_feature(99);
        assert_eq!(state.selected_feature, 0);
        state.select_feature(2);
        assert_eq!(state.selected_feature, 2);
    }

    #[test]
    fn hold_progress_increases_while_holding() {
        let mut state = OnboardingUiState::new();
        state.orb_pressed();
        let now = state.now + std::time::Duration::from_millis(200);
        state.tick(now);
        assert!(state.hold_progress > 0.0);
    }
}
