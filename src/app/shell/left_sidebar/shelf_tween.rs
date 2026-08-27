//! Manual height tweens for collapsible thread shelves (pinned / settled / archived).
//!
//! GPUI has no CSS transitions. Clip height is eased over 200ms with the same
//! resize ease-out as dock panel toggles, evaluated each render.

use std::time::{Duration, Instant};

use crate::shared::theme::ease_out_resize;

use super::tokens::{ROW_HEIGHT_CARD, ROW_HEIGHT_SLIM, SHELF_ROW_GAP, SHOW_MORE_HEIGHT};

/// Shelf collapse/expand duration — matches dock panel toggles.
pub const SHELF_TOGGLE_DURATION: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_millis(200)
};

#[derive(Debug, Clone, Copy)]
pub struct ShelfHeightTween {
    pub from: f32,
    pub to: f32,
    pub started_at: Instant,
}

impl ShelfHeightTween {
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
        now.saturating_duration_since(self.started_at) < SHELF_TOGGLE_DURATION
    }
}

pub fn eval_shelf_tween(tween: Option<ShelfHeightTween>, target: f32, now: Instant) -> f32 {
    let Some(tween) = tween else {
        return target;
    };
    if !tween.is_active(now) {
        return target;
    }
    let total = SHELF_TOGGLE_DURATION.as_secs_f32();
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

pub fn shelf_rows_height(row_count: usize, row_height: f32) -> f32 {
    if row_count == 0 {
        return 0.0;
    }
    let rows = row_count as f32 * row_height;
    let gaps = (row_count - 1) as f32 * SHELF_ROW_GAP;
    rows + gaps
}

pub fn shelf_content_height(row_count: usize, show_more: bool, row_height: f32) -> f32 {
    let mut height = shelf_rows_height(row_count, row_height);
    if show_more {
        height += SHOW_MORE_HEIGHT;
    }
    height
}

pub fn shelf_content_height_slim(row_count: usize, show_more: bool) -> f32 {
    shelf_content_height(row_count, show_more, ROW_HEIGHT_SLIM)
}

pub fn shelf_content_height_card(row_count: usize) -> f32 {
    shelf_content_height(row_count, false, ROW_HEIGHT_CARD)
}

pub fn shelf_expand_progress(clip_height: f32, full_height: f32) -> f32 {
    if full_height <= f32::EPSILON {
        return 1.0;
    }
    (clip_height / full_height).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_endpoints_match_from_and_target() {
        let now = Instant::now();
        let tween = ShelfHeightTween::new(108.0, 0.0, now);
        if SHELF_TOGGLE_DURATION.is_zero() {
            assert!((eval_shelf_tween(Some(tween), 0.0, now) - 0.0).abs() < 1e-5);
            return;
        }
        assert!((eval_shelf_tween(Some(tween), 0.0, now) - 108.0).abs() < 1e-5);
        let done = now + SHELF_TOGGLE_DURATION;
        assert!((eval_shelf_tween(Some(tween), 0.0, done) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn rows_height_accounts_for_gap() {
        assert_eq!(shelf_rows_height(0, ROW_HEIGHT_SLIM), 0.0);
        assert_eq!(shelf_rows_height(1, ROW_HEIGHT_SLIM), ROW_HEIGHT_SLIM);
        assert_eq!(
            shelf_rows_height(2, ROW_HEIGHT_SLIM),
            ROW_HEIGHT_SLIM * 2.0 + SHELF_ROW_GAP
        );
        assert_eq!(
            shelf_content_height_card(2),
            ROW_HEIGHT_CARD * 2.0 + SHELF_ROW_GAP
        );
    }
}
