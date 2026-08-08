//! Interactive onboarding UI state (animation, orb hold).

use std::sync::Arc;
use std::time::Instant;

use gpui::{App, FocusHandle, Window};

use super::onboarding_dynamics::dynamics_for_progress;
use super::onboarding_galaxy_orb::GalaxyParticleCache;
use crate::shared::theme::OpenCoreTheme;

/// Local onboarding animation state (GPU-free).
#[derive(Debug, Clone)]
pub struct OnboardingUiState {
    pub started_at: Instant,
    pub now: Instant,
    pub is_holding: bool,
    pub hold_progress: f32,
    pub displayed_speed: f32,
    pub displayed_zoom: f32,
    /// Visual press feedback for the primary CTA button.
    pub cta_pressed: bool,
    focus_claimed: bool,
    particle_cache: Option<Arc<GalaxyParticleCache>>,
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
            cta_pressed: false,
            focus_claimed: false,
            particle_cache: None,
        }
    }

    pub fn ensure_particle_cache(&mut self, theme: OpenCoreTheme) {
        let needs_bake = self
            .particle_cache
            .as_ref()
            .map(|c| c.theme() != theme)
            .unwrap_or(true);
        if needs_bake {
            self.particle_cache = Some(Arc::new(GalaxyParticleCache::bake(theme)));
        }
    }

    pub fn particle_cache_arc(&self) -> Arc<GalaxyParticleCache> {
        Arc::clone(
            self.particle_cache
                .as_ref()
                .expect("ensure_particle_cache must run before paint"),
        )
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

    /// Marks the primary CTA as pressed (mouse down) for visual feedback.
    pub fn cta_pressed(&mut self) {
        self.cta_pressed = true;
    }

    /// Clears the CTA pressed state (mouse up).
    pub fn cta_released(&mut self) {
        self.cta_pressed = false;
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

    #[test]
    fn ensure_particle_cache_bakes_once_per_theme() {
        use crate::shared::theme::{OpenCoreTheme, ThemeMode};

        let mut state = OnboardingUiState::new();
        let dark = OpenCoreTheme::resolve(ThemeMode::Dark);
        state.ensure_particle_cache(dark);
        let first = state.particle_cache_arc();
        state.ensure_particle_cache(dark);
        assert!(Arc::ptr_eq(&first, &state.particle_cache_arc()));

        let light = OpenCoreTheme::resolve(ThemeMode::Light);
        state.ensure_particle_cache(light);
        assert_ne!(state.particle_cache_arc().theme(), dark);
    }
}
