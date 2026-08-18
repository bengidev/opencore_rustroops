//! Persisted shell chrome sizes and open flags.

pub use crate::shared::preferences::shell_chrome::{
    BOTTOM_DEFAULT, BOTTOM_MAX_VH, BOTTOM_MIN, RIGHT_DEFAULT, RIGHT_MAX, RIGHT_MIN,
    SIDEBAR_DEFAULT, SIDEBAR_MAX, SIDEBAR_MIN, ShellChrome, ShellTabRecord, TITLEBAR_HEIGHT,
    clamp_bottom_height, clamp_right_width, clamp_sidebar_width,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_plan() {
        assert_eq!(TITLEBAR_HEIGHT, 38.0);
        assert_eq!(SIDEBAR_MIN, 208.0);
        assert_eq!(SIDEBAR_MAX, 400.0);
        assert_eq!(SIDEBAR_DEFAULT, 256.0);
        assert_eq!(RIGHT_MIN, 240.0);
        assert_eq!(RIGHT_MAX, 480.0);
        assert_eq!(RIGHT_DEFAULT, 320.0);
        assert_eq!(BOTTOM_MIN, 120.0);
        assert_eq!(BOTTOM_DEFAULT, 220.0);
        assert_eq!(BOTTOM_MAX_VH, 0.55);
    }

    #[test]
    fn clamp_sidebar_respects_min_max() {
        assert_eq!(clamp_sidebar_width(10.0), SIDEBAR_MIN);
        assert_eq!(clamp_sidebar_width(999.0), SIDEBAR_MAX);
        assert_eq!(clamp_sidebar_width(300.0), 300.0);
    }

    #[test]
    fn clamp_right_respects_min_max_and_viewport_fraction() {
        assert_eq!(clamp_right_width(10.0, 1280.0), RIGHT_MIN);
        assert_eq!(
            clamp_right_width(900.0, 1280.0),
            (1280.0_f32 * 0.52).min(RIGHT_MAX)
        );
        assert_eq!(clamp_right_width(300.0, 1280.0), 300.0);
    }

    #[test]
    fn clamp_bottom_respects_min_and_viewport_fraction() {
        assert_eq!(clamp_bottom_height(10.0, 800.0), BOTTOM_MIN);
        assert_eq!(clamp_bottom_height(900.0, 800.0), 800.0 * BOTTOM_MAX_VH);
        assert_eq!(clamp_bottom_height(200.0, 800.0), 200.0);
    }

    #[test]
    fn shell_chrome_default_matches_spec() {
        let c = ShellChrome::default();
        assert!(c.left_open);
        assert!(!c.right_open);
        assert!(!c.bottom_open);
        assert_eq!(c.left_width, SIDEBAR_DEFAULT);
        assert_eq!(c.right_width, RIGHT_DEFAULT);
        assert_eq!(c.bottom_height, BOTTOM_DEFAULT);
        assert_eq!(c.tabs.len(), 1);
        assert_eq!(c.active_tab_id, c.tabs[0].id);
    }
}
