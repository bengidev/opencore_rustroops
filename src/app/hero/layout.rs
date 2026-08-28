//! Hero layout math — welcome center, docked title-bar slot (layout A).

use super::brand::{BRAND_ASPECT, brand_width};
use crate::app::state::{HOME_WINDOW_HEIGHT, HOME_WINDOW_WIDTH};
use crate::app::viewport::WindowViewport;
use gpui_component::TITLE_BAR_HEIGHT;

pub const BRAND_HERO_MIN: f32 = 220.0;
pub const BRAND_HERO_MAX: f32 = 320.0;
/// Shell title-bar brand height.
pub const BRAND_SHELL_HEIGHT: f32 = 18.0;

/// Ghost xsmall icon button width in the shell title bar.
pub const SHELL_TOGGLE_WIDTH: f32 = 28.0;
pub const SHELL_TITLE_GAP: f32 = 4.0;

const WELCOME_EDGE_INSET_H: f32 = 16.0;
const WELCOME_HEADER_BAND: f32 = 46.0;
const WELCOME_ACTION_BAND: f32 = 260.0;

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

/// Center of the large welcome brand in window coordinates.
pub fn welcome_brand_center(viewport: WindowViewport) -> (f32, f32) {
    let hero_height = responsive_brand_height(viewport);
    let content_top = WELCOME_HEADER_BAND;
    let content_height = (viewport.height - content_top - WELCOME_ACTION_BAND).max(hero_height);
    let center_y = content_top + content_height * 0.5;
    (viewport.width * 0.5, center_y)
}

/// Responsive welcome brand height.
pub fn responsive_brand_height(viewport: WindowViewport) -> f32 {
    let square_limit = responsive_hero_size(viewport.width, viewport.height);
    let width_limit = (viewport.width - WELCOME_EDGE_INSET_H * 2.0) / BRAND_ASPECT;
    square_limit
        .min(width_limit)
        .min(BRAND_HERO_MAX / BRAND_ASPECT)
}

/// Center of the docked brand: `[toggle-left] [brand lockup]` (layout A).
pub fn docked_brand_center(viewport: WindowViewport) -> (f32, f32) {
    let title_h = TITLE_BAR_HEIGHT.as_f32();
    let width = brand_width(BRAND_SHELL_HEIGHT);
    let x = title_bar_left_padding() + SHELL_TOGGLE_WIDTH + SHELL_TITLE_GAP + width * 0.5;
    let _ = viewport;
    (x, title_h * 0.5)
}

/// Viewport used to compute the docked slot after welcome completes.
pub fn home_transition_viewport() -> WindowViewport {
    WindowViewport {
        width: HOME_WINDOW_WIDTH as f32,
        height: HOME_WINDOW_HEIGHT as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docked_brand_sits_after_left_toggle() {
        let (x, y) = docked_brand_center(home_transition_viewport());
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
}
