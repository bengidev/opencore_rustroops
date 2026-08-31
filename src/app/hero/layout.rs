//! Hero layout math — welcome center, docked title-bar slot (layout A).

use super::brand::BRAND_ASPECT;
use crate::app::viewport::WindowViewport;
use crate::shared::theme::TypeRole;

pub const BRAND_HERO_MIN: f32 = 220.0;
pub const BRAND_HERO_MAX: f32 = 320.0;
/// Shell title-bar brand height.
pub const BRAND_SHELL_HEIGHT: f32 = 18.0;

/// Ghost xsmall icon button width in the shell title bar.
#[allow(dead_code)]
pub const SHELL_TOGGLE_WIDTH: f32 = 28.0;
#[allow(dead_code)]
pub const SHELL_TITLE_GAP: f32 = 4.0;

pub const WELCOME_EDGE_INSET_H: f32 = 16.0;
pub const WELCOME_TITLEBAR_HEIGHT: f32 = 38.0;
pub const WELCOME_EDGE_INSET_TOP: f32 = 4.0;
pub const WELCOME_EDGE_INSET_BOTTOM: f32 = 20.0;
pub const WELCOME_HERO_BRAND_FRAME_EXTRA: f32 = 40.0;
pub const WELCOME_ACTION_SPACER: f32 = 60.0;
pub const WELCOME_ENTER_BUTTON_HEIGHT: f32 = 48.0;

const WELCOME_ACTION_BAND: f32 = 260.0;
const WELCOME_HEADER_GAP: f32 = 8.0;
const WELCOME_THEME_TOGGLE_HEIGHT: f32 = 32.0;

/// macOS traffic-light inset matches gpui-component `TITLE_BAR_LEFT_PADDING`.
pub fn title_bar_left_padding() -> f32 {
    #[cfg(target_os = "macos")]
    {
        80.0
    }
    #[cfg(not(target_os = "macos"))]
    {
        12.0
    }
}

/// Responsive welcome hero size (ported from welcome view constants).
pub fn responsive_hero_size(available_width: f32, available_height: f32) -> f32 {
    let width_limit = (available_width - WELCOME_EDGE_INSET_H * 2.0).max(BRAND_HERO_MIN);
    let height_limit = (available_height - WELCOME_ACTION_BAND).max(BRAND_HERO_MIN);
    width_limit.min(height_limit).min(BRAND_HERO_MAX)
}

/// Height of the welcome header row (title + subtitle).
pub fn welcome_header_row_height() -> f32 {
    let text_column = TypeRole::LabelMd.size() * TypeRole::LabelMd.line_height()
        + 2.0
        + TypeRole::MonoSm.size() * TypeRole::MonoSm.line_height();
    text_column.max(WELCOME_THEME_TOGGLE_HEIGHT)
}

/// Vertical space available for the centered brand frame and copy column.
pub fn welcome_vertical_content_budget(viewport: WindowViewport) -> f32 {
    (viewport.height
        - WELCOME_EDGE_INSET_TOP
        - WELCOME_EDGE_INSET_BOTTOM
        - welcome_header_row_height()
        - WELCOME_HEADER_GAP
        - WELCOME_ACTION_BAND)
        .max(0.0)
}

/// Responsive welcome brand height.
pub fn responsive_brand_height(viewport: WindowViewport) -> f32 {
    let square_limit = responsive_hero_size(viewport.width, viewport.height);
    let width_limit = (viewport.width - WELCOME_EDGE_INSET_H * 2.0) / BRAND_ASPECT;
    square_limit
        .min(width_limit)
        .min(BRAND_HERO_MAX / BRAND_ASPECT)
}

/// Large centered brand height during the show-off phase.
///
/// Unlike the old wireframe cube (square), the brand lockup is very wide
/// (`BRAND_ASPECT`), so height must be derived from available width.
pub fn show_off_brand_height(viewport: WindowViewport) -> f32 {
    let hero = responsive_brand_height(viewport);
    let width_limit = (viewport.width - WELCOME_EDGE_INSET_H * 2.0) / BRAND_ASPECT;
    let available_height = welcome_vertical_content_budget(viewport);
    // Prominent intro size that still fits the viewport; morphs down to `hero`.
    width_limit
        .min(available_height * 0.4)
        .max(hero)
}

/// Linear interpolation between two values.
pub fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::hero::brand::brand_width;
    use gpui_component::TITLE_BAR_HEIGHT;

    /// Center of the docked brand: `[toggle-left] [brand lockup]` (layout A).
    fn docked_brand_center(viewport: WindowViewport) -> (f32, f32) {
        let title_h = TITLE_BAR_HEIGHT.as_f32();
        let width = brand_width(BRAND_SHELL_HEIGHT);
        let x = title_bar_left_padding() + SHELL_TOGGLE_WIDTH + SHELL_TITLE_GAP + width * 0.5;
        let _ = viewport;
        (x, title_h * 0.5)
    }

    #[test]
    fn docked_brand_sits_after_left_toggle() {
        let viewport = WindowViewport {
            width: 1280.0,
            height: 800.0,
        };
        let (x, y) = docked_brand_center(viewport);
        let width = brand_width(BRAND_SHELL_HEIGHT);
        let expected_x =
            title_bar_left_padding() + SHELL_TOGGLE_WIDTH + SHELL_TITLE_GAP + width * 0.5;
        assert!((x - expected_x).abs() < 1e-3);
        assert!((y - TITLE_BAR_HEIGHT.as_f32() * 0.5).abs() < 1e-3);
    }

    #[test]
    fn welcome_hero_size_clamps_on_narrow_windows() {
        assert_eq!(responsive_hero_size(440.0, 360.0), BRAND_HERO_MIN);
        assert_eq!(responsive_hero_size(1200.0, 900.0), BRAND_HERO_MAX);
    }

    #[test]
    fn show_off_brand_fits_within_viewport_width() {
        let viewport = WindowViewport {
            width: 960.0,
            height: 740.0,
        };
        let height = show_off_brand_height(viewport);
        let width = brand_width(height);
        assert!(width <= viewport.width - WELCOME_EDGE_INSET_H * 2.0 + 1.0);
        assert!(height >= responsive_brand_height(viewport));
    }

    #[test]
    fn show_off_brand_fits_narrow_viewport() {
        let viewport = WindowViewport {
            width: 440.0,
            height: 360.0,
        };
        let height = show_off_brand_height(viewport);
        let width = brand_width(height);
        assert!(width <= viewport.width - WELCOME_EDGE_INSET_H * 2.0 + 1.0);
        assert!(height >= responsive_brand_height(viewport));
    }

    #[test]
    fn show_off_brand_settles_to_resting_height() {
        let viewport = WindowViewport {
            width: 960.0,
            height: 740.0,
        };
        let hero = responsive_brand_height(viewport);
        let show_off = show_off_brand_height(viewport);
        let settled = lerp_f32(show_off, hero, 1.0);
        assert!((settled - hero).abs() < 1e-3);
    }

    #[test]
    fn show_off_brand_fits_within_viewport_height() {
        let viewport = WindowViewport {
            width: 960.0,
            height: 740.0,
        };
        let height = show_off_brand_height(viewport);
        let available = welcome_vertical_content_budget(viewport);
        assert!(height + WELCOME_HERO_BRAND_FRAME_EXTRA <= available + 1.0);
    }

    #[test]
    fn show_off_brand_fits_short_viewport_height() {
        let viewport = WindowViewport {
            width: 960.0,
            height: 500.0,
        };
        let height = show_off_brand_height(viewport);
        let available = welcome_vertical_content_budget(viewport);
        assert!(height + WELCOME_HERO_BRAND_FRAME_EXTRA <= available + 1.0);
    }
}
