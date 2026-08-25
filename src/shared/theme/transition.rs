//! Interruptible light/dark color morph.
//!
//! GPUI has no CSS transitions. This is the port of a color transition:
//! retargetable mix in `[0, 1]` (`0` = dark, `1` = light), strong ease-out,
//! 220ms. Color only — no transform — so it stays valid under reduced motion.

use std::time::{Duration, Instant};

use super::ThemeMode;

/// CSS `cubic-bezier(0.23, 1, 0.32, 1)` — Emil's strong ease-out.
const EASE_OUT_X1: f32 = 0.23;
const EASE_OUT_Y1: f32 = 1.0;
const EASE_OUT_X2: f32 = 0.32;
const EASE_OUT_Y2: f32 = 1.0;

/// Theme morph duration. Full-window color change; stays under 300ms.
pub const THEME_TRANSITION_DURATION: Duration = Duration::from_millis(220);

/// Mix toward light (`0` = dark palette, `1` = light palette).
pub const fn mix_light_for(mode: ThemeMode) -> f32 {
    match mode {
        ThemeMode::Dark => 0.0,
        ThemeMode::Light => 1.0,
    }
}

/// Strong ease-out: starts immediately, settles at the end.
pub fn ease_out_strong(t: f32) -> f32 {
    unit_bezier(
        EASE_OUT_X1,
        EASE_OUT_Y1,
        EASE_OUT_X2,
        EASE_OUT_Y2,
        t.clamp(0.0, 1.0),
    )
}

/// Standard ease-out for panel resize — comet `EASE_OUT` / `cubic-bezier(0, 0, 0.58, 1)`.
pub fn ease_out_resize(t: f32) -> f32 {
    unit_bezier(0.0, 0.0, 0.58, 1.0, t.clamp(0.0, 1.0))
}

/// In-flight theme morph. Retargets from the current mix when toggled mid-way.
#[derive(Debug, Clone, Copy)]
pub struct ThemeTransition {
    from_mix: f32,
    to_mix: f32,
    started_at: Instant,
    duration: Duration,
}

impl ThemeTransition {
    pub fn start(from: ThemeMode, to: ThemeMode, now: Instant) -> Self {
        Self {
            from_mix: mix_light_for(from),
            to_mix: mix_light_for(to),
            started_at: now,
            duration: THEME_TRANSITION_DURATION,
        }
    }

    /// CSS-style retarget: continue from the current visual mix toward `to`.
    pub fn retarget(&mut self, to: ThemeMode, now: Instant) {
        self.from_mix = self.mix_light(now);
        self.to_mix = mix_light_for(to);
        self.started_at = now;
    }

    pub fn mix_light(&self, now: Instant) -> f32 {
        let t = self.linear_progress(now);
        lerp_f32(self.from_mix, self.to_mix, ease_out_strong(t))
    }

    pub fn is_active(&self, now: Instant) -> bool {
        (self.from_mix - self.to_mix).abs() > f32::EPSILON && self.linear_progress(now) < 1.0
    }

    fn linear_progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started_at);
        let total = self.duration.as_secs_f32();
        if total <= 0.0 {
            1.0
        } else {
            (elapsed.as_secs_f32() / total).clamp(0.0, 1.0)
        }
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn unit_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let mut guess = x;
    for _ in 0..8 {
        let current_x = sample_bezier(guess, x1, x2);
        let delta = current_x - x;
        if delta.abs() < 1e-6 {
            break;
        }
        let derivative = sample_bezier_derivative(guess, x1, x2);
        if derivative.abs() < 1e-6 {
            break;
        }
        guess = (guess - delta / derivative).clamp(0.0, 1.0);
    }
    sample_bezier(guess, y1, y2)
}

fn sample_bezier(t: f32, p1: f32, p2: f32) -> f32 {
    let one_t = 1.0 - t;
    3.0 * one_t * one_t * t * p1 + 3.0 * one_t * t * t * p2 + t * t * t
}

fn sample_bezier_derivative(t: f32, p1: f32, p2: f32) -> f32 {
    let one_t = 1.0 - t;
    3.0 * one_t * one_t * p1 + 6.0 * one_t * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_is_under_300ms() {
        assert_eq!(THEME_TRANSITION_DURATION, Duration::from_millis(220));
        assert!(THEME_TRANSITION_DURATION < Duration::from_millis(300));
    }

    #[test]
    fn ease_out_endpoints() {
        assert_eq!(ease_out_strong(0.0), 0.0);
        assert_eq!(ease_out_strong(1.0), 1.0);
    }

    #[test]
    fn ease_out_starts_fast() {
        let mid_early = ease_out_strong(0.2);
        assert!(
            mid_early > 0.2,
            "ease-out must move immediately; got {mid_early}"
        );
    }

    #[test]
    fn mix_light_endpoints() {
        assert_eq!(mix_light_for(ThemeMode::Dark), 0.0);
        assert_eq!(mix_light_for(ThemeMode::Light), 1.0);
    }

    #[test]
    fn start_begins_at_from_palette() {
        let now = Instant::now();
        let tx = ThemeTransition::start(ThemeMode::Dark, ThemeMode::Light, now);
        assert!((tx.mix_light(now) - 0.0).abs() < 1e-5);
        assert!(tx.is_active(now));
    }

    #[test]
    fn start_settles_at_to_palette() {
        let now = Instant::now();
        let tx = ThemeTransition::start(ThemeMode::Dark, ThemeMode::Light, now);
        let done = now + THEME_TRANSITION_DURATION;
        assert!((tx.mix_light(done) - 1.0).abs() < 1e-5);
        assert!(!tx.is_active(done));
    }

    #[test]
    fn retarget_continues_from_current_mix() {
        let now = Instant::now();
        let mut tx = ThemeTransition::start(ThemeMode::Dark, ThemeMode::Light, now);
        let mid = now + THEME_TRANSITION_DURATION / 2;
        let before = tx.mix_light(mid);
        assert!(before > 0.4, "ease-out should be well along by halfway");
        tx.retarget(ThemeMode::Dark, mid);
        assert!((tx.mix_light(mid) - before).abs() < 1e-5);
        let done = mid + THEME_TRANSITION_DURATION;
        assert!((tx.mix_light(done) - 0.0).abs() < 1e-5);
    }
}
