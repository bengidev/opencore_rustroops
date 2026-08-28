//! Big-to-small brand hero transition (iOS onboarding port).

use std::time::{Duration, Instant};

use super::layout::{
    BRAND_SHELL_HEIGHT, docked_brand_center, home_transition_viewport, welcome_brand_center,
};
use crate::app::viewport::WindowViewport;

/// Hero morph duration — matches iOS `smooth(duration: 1.02)`.
pub const HERO_TRANSITION_DURATION: Duration = Duration::from_millis(1020);

const MORPH_START: f32 = 0.54;

#[derive(Debug, Clone, Copy)]
pub struct HeroTransition {
    started_at: Instant,
    start_center: (f32, f32),
    start_size: f32,
    end_center: (f32, f32),
    end_size: f32,
}

impl HeroTransition {
    pub fn start(now: Instant, welcome_viewport: WindowViewport, hero_size: f32) -> Self {
        let start_center = welcome_brand_center(welcome_viewport);
        let end_center = docked_brand_center(home_transition_viewport());
        Self {
            started_at: now,
            start_center,
            start_size: hero_size,
            end_center,
            end_size: BRAND_SHELL_HEIGHT,
        }
    }

    pub fn linear_progress(&self, now: Instant) -> f32 {
        let total = HERO_TRANSITION_DURATION.as_secs_f32();
        if total <= 0.0 {
            return 1.0;
        }
        (now.saturating_duration_since(self.started_at).as_secs_f32() / total).clamp(0.0, 1.0)
    }

    pub fn is_active(&self, now: Instant) -> bool {
        self.linear_progress(now) < 1.0
    }

    pub fn morph_progress(transition: f32) -> f32 {
        let span = (1.0 - MORPH_START).max(0.001);
        let raw = ((transition - MORPH_START) / span).clamp(0.0, 1.0);
        1.0 - (1.0 - raw).powi(3)
    }

    /// Window-space center and size at `now`.
    pub fn layout_at(&self, now: Instant) -> (f32, f32, f32) {
        let transition = self.linear_progress(now);
        let morph = Self::morph_progress(transition);
        let cx = lerp(self.start_center.0, self.end_center.0, morph);
        let cy = lerp(self.start_center.1, self.end_center.1, morph);
        let size = lerp(self.start_size, self.end_size, morph);
        (cx, cy, size)
    }

    /// Fades welcome chrome out during the first third of the transition.
    pub fn content_opacity(transition: f32) -> f32 {
        (1.0 - (transition / 0.35).clamp(0.0, 1.0)).max(0.0)
    }

    pub fn shell_brand_opacity(transition: f32) -> f32 {
        if transition >= 1.0 {
            1.0
        } else {
            ((transition - 0.72) / 0.28).clamp(0.0, 1.0)
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_opacity_visible_before_transition() {
        assert!((HeroTransition::content_opacity(0.0) - 1.0).abs() < 1e-3);
        assert!(HeroTransition::content_opacity(1.0) < 0.01);
    }

    #[test]
    fn morph_reaches_completion_at_end() {
        assert!(HeroTransition::morph_progress(0.38) < 0.01);
        assert!(HeroTransition::morph_progress(1.0) >= 0.99);
    }

    #[test]
    fn transition_endpoints() {
        let now = Instant::now();
        let tx = HeroTransition::start(
            now,
            WindowViewport {
                width: 960.0,
                height: 740.0,
            },
            220.0,
        );
        let (sx, sy, ss) = tx.layout_at(now);
        assert!((sx - tx.start_center.0).abs() < 1e-3);
        assert!((sy - tx.start_center.1).abs() < 1e-3);
        assert!((ss - 220.0).abs() < 1e-3);

        let done = now + HERO_TRANSITION_DURATION;
        let (ex, ey, es) = tx.layout_at(done);
        assert!((ex - tx.end_center.0).abs() < 1e-3);
        assert!((ey - tx.end_center.1).abs() < 1e-3);
        assert!((es - BRAND_SHELL_HEIGHT).abs() < 1e-3);
    }
}
