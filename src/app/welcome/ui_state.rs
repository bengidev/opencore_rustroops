//! Interactive welcome UI state (keyboard focus, intro animation).

use std::time::{Duration, Instant};

use gpui::{App, FocusHandle, Window};

/// Brand morph + header, copy, and CTA fade-in on welcome load.
pub const CHROME_REVEAL_DURATION: Duration = Duration::from_millis(800);

pub struct WelcomeUiState {
    focus_claimed: bool,
    started_at: Option<Instant>,
}

impl WelcomeUiState {
    pub fn new() -> Self {
        Self {
            focus_claimed: false,
            started_at: None,
        }
    }

    /// Advances intro timing and returns true while the reveal animation is active.
    pub fn tick(&mut self, now: Instant) -> bool {
        self.started_at.get_or_insert(now);
        self.intro_animating(now)
    }

    pub fn intro_animating(&self, now: Instant) -> bool {
        match self.started_at {
            None => true,
            Some(started_at) => {
                now.saturating_duration_since(started_at) < CHROME_REVEAL_DURATION
            }
        }
    }

    pub fn reveal_progress(&self, now: Instant) -> f32 {
        let Some(started_at) = self.started_at else {
            return 0.0;
        };
        let elapsed = now.saturating_duration_since(started_at).as_secs_f32();
        let duration = CHROME_REVEAL_DURATION.as_secs_f32();
        if duration <= 0.0 {
            return 1.0;
        }
        let t = (elapsed / duration).clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    pub fn chrome_opacity(&self, now: Instant) -> f32 {
        self.reveal_progress(now)
    }

    pub fn accepts_enter(&self) -> bool {
        true
    }

    /// Requests keyboard focus once per welcome session.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_opacity_eases_in_from_start() {
        let start = Instant::now();
        let ui = WelcomeUiState {
            focus_claimed: false,
            started_at: Some(start),
        };
        assert!((ui.chrome_opacity(start) - 0.0).abs() < 1e-3);
        assert!(ui.chrome_opacity(start + CHROME_REVEAL_DURATION) >= 0.99);
    }

    #[test]
    fn intro_animating_only_during_reveal() {
        let start = Instant::now();
        let ui = WelcomeUiState {
            focus_claimed: false,
            started_at: Some(start),
        };
        assert!(ui.intro_animating(start));
        assert!(!ui.intro_animating(start + CHROME_REVEAL_DURATION));
    }
}
