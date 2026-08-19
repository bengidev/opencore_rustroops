//! Persisted shell chrome sizes and open flags.

pub use crate::shared::preferences::shell_chrome::{
    BOTTOM_DEFAULT, RIGHT_DEFAULT, SIDEBAR_DEFAULT, ShellChrome, ShellTabRecord, TITLEBAR_HEIGHT,
    clamp_bottom_height, clamp_right_width, clamp_sidebar_width,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_plan() {
        assert_eq!(TITLEBAR_HEIGHT, 38.0);
        assert_eq!(SIDEBAR_DEFAULT, 256.0);
        assert_eq!(RIGHT_DEFAULT, 320.0);
        assert_eq!(BOTTOM_DEFAULT, 220.0);
    }

    #[test]
    fn clamp_sidebar_only_floors_at_zero() {
        assert_eq!(clamp_sidebar_width(-10.0), 0.0);
        assert_eq!(clamp_sidebar_width(10.0), 10.0);
        assert_eq!(clamp_sidebar_width(999.0), 999.0);
    }

    #[test]
    fn clamp_right_floors_at_zero_and_caps_at_viewport() {
        assert_eq!(clamp_right_width(-10.0, 1280.0), 0.0);
        assert_eq!(clamp_right_width(900.0, 1280.0), 900.0);
        assert_eq!(clamp_right_width(900.0, 800.0), 800.0);
        assert_eq!(clamp_right_width(300.0, 1280.0), 300.0);
    }

    #[test]
    fn clamp_bottom_floors_at_zero_and_caps_at_viewport() {
        assert_eq!(clamp_bottom_height(-10.0, 800.0), 0.0);
        assert_eq!(clamp_bottom_height(900.0, 800.0), 800.0);
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

    #[test]
    fn persisted_sanitization_keeps_large_sizes_for_live_viewport_caps() {
        let chrome = ShellChrome {
            right_open: true,
            right_width: 400.0,
            bottom_open: true,
            bottom_height: 400.0,
            ..Default::default()
        }
        .sanitized_persisted();

        assert_eq!(chrome.right_width, 400.0);
        assert_eq!(chrome.bottom_height, 400.0);
        assert_eq!(
            crate::app::shell::Shell::right_target_for_viewport(&chrome, 300.0),
            300.0
        );
        assert_eq!(
            crate::app::shell::Shell::bottom_target_for_viewport(&chrome, 100.0),
            100.0
        );
    }
}
