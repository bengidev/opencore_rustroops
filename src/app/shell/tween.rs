//! Time-based interpolation for shell panel dimensions.

use std::time::Instant;

/// Duration of a shell panel resize animation, in milliseconds.
pub const RESIZE_MS: f32 = 200.0;

/// An in-flight dimension interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimTween {
    pub from: f32,
    pub to: f32,
    pub started: Instant,
}

/// Cubic ease-out (`1 - (1 - t)^3`) over a unit interval.
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Evaluate an optional resize tween at `now`.
///
/// A missing tween and reduced-motion mode both use the current target directly.
/// Once a tween is present, its own `to` endpoint is authoritative for the
/// duration of that animation.
pub fn eval_tween(
    tween: Option<&DimTween>,
    target: f32,
    now: Instant,
    reduced_motion: bool,
) -> f32 {
    if reduced_motion {
        return target;
    }

    let Some(tween) = tween else {
        return target;
    };

    let progress = (now.saturating_duration_since(tween.started).as_secs_f32() * 1000.0
        / RESIZE_MS)
        .clamp(0.0, 1.0);
    tween.from + (tween.to - tween.from) * ease_out(progress)
}

/// Return whether a resize tween has reached its duration.
pub fn tween_finished(tween: &DimTween, now: Instant) -> bool {
    now.saturating_duration_since(tween.started).as_secs_f32() * 1000.0 >= RESIZE_MS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn eval_at_start_is_from() {
        let started = Instant::now();
        let tween = DimTween {
            from: 120.0,
            to: 320.0,
            started,
        };

        assert_eq!(
            eval_tween(Some(&tween), tween.to, started, false),
            tween.from
        );
    }

    #[test]
    fn eval_at_end_is_to() {
        let started = Instant::now();
        let tween = DimTween {
            from: 120.0,
            to: 320.0,
            started,
        };

        assert_eq!(
            eval_tween(
                Some(&tween),
                tween.to,
                started + Duration::from_millis(200),
                false,
            ),
            tween.to
        );
    }

    #[test]
    fn reduced_motion_snaps_to_target() {
        let started = Instant::now();
        let tween = DimTween {
            from: 120.0,
            to: 320.0,
            started,
        };

        assert_eq!(
            eval_tween(
                Some(&tween),
                400.0,
                started + Duration::from_millis(50),
                true
            ),
            400.0
        );
    }

    #[test]
    fn eval_midpoint_uses_cubic_ease_out() {
        let started = Instant::now();
        let tween = DimTween {
            from: 100.0,
            to: 300.0,
            started,
        };
        let midpoint = eval_tween(
            Some(&tween),
            tween.to,
            started + Duration::from_millis(100),
            false,
        );

        assert_eq!(midpoint, 275.0);
    }

    #[test]
    fn absent_tween_returns_target() {
        let now = Instant::now();

        assert_eq!(eval_tween(None, 240.0, now, false), 240.0);
    }

    #[test]
    fn tween_is_finished_at_and_after_duration() {
        let started = Instant::now();
        let tween = DimTween {
            from: 100.0,
            to: 300.0,
            started,
        };

        assert!(!tween_finished(
            &tween,
            started + Duration::from_millis(199)
        ));
        assert!(tween_finished(&tween, started + Duration::from_millis(200)));
        assert!(tween_finished(&tween, started + Duration::from_millis(201)));
    }

    #[test]
    fn ease_out_has_unit_endpoints_and_clamps() {
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert_eq!(ease_out(-1.0), 0.0);
        assert_eq!(ease_out(2.0), 1.0);
    }
}
