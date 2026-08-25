//! Manual dock size tweens for panel show/hide (comet / zeron pattern).
//!
//! GPUI has no CSS transitions. Width and height are eased over 200ms with a
//! standard ease-out, evaluated each render — never via `with_animation`, so
//! remounting ancestors does not replay the tween from t=0.

use std::time::{Duration, Instant};

use crate::shared::theme::ease_out_resize;

/// Panel collapse/expand duration — matches comet [`RESIZE`].
pub const DOCK_TOGGLE_DURATION: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_millis(200)
};

/// A oneshot size tween driven manually from [`ShellWorkspace::render`].
#[derive(Debug, Clone, Copy)]
pub struct DockSizeTween {
    pub from: f32,
    pub to: f32,
    pub started_at: Instant,
}

impl DockSizeTween {
    pub fn new(from: f32, to: f32, now: Instant) -> Self {
        Self {
            from,
            to,
            started_at: now,
        }
    }

    pub fn is_active(&self, now: Instant) -> bool {
        if (self.from - self.to).abs() <= f32::EPSILON {
            return false;
        }
        now.saturating_duration_since(self.started_at) < DOCK_TOGGLE_DURATION
    }
}

/// Evaluate a dock tween at `now`. Finished or absent tweens return `target`.
pub fn eval_dock_tween(tween: Option<DockSizeTween>, target: f32, now: Instant) -> f32 {
    let Some(tween) = tween else {
        return target;
    };
    if !tween.is_active(now) {
        return target;
    }
    let total = DOCK_TOGGLE_DURATION.as_secs_f32();
    if total <= 0.0 {
        return target;
    }
    let raw = now
        .saturating_duration_since(tween.started_at)
        .as_secs_f32()
        / total;
    let t = ease_out_resize(raw.clamp(0.0, 1.0));
    tween.from + (tween.to - tween.from) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_endpoints_match_from_and_target() {
        let now = Instant::now();
        let tween = DockSizeTween::new(256.0, 0.0, now);
        if DOCK_TOGGLE_DURATION.is_zero() {
            assert!((eval_dock_tween(Some(tween), 0.0, now) - 0.0).abs() < 1e-5);
            return;
        }
        assert!((eval_dock_tween(Some(tween), 0.0, now) - 256.0).abs() < 1e-5);
        let done = now + DOCK_TOGGLE_DURATION;
        assert!((eval_dock_tween(Some(tween), 0.0, done) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn absent_tween_returns_target() {
        let now = Instant::now();
        assert_eq!(eval_dock_tween(None, 320.0, now), 320.0);
    }
}
