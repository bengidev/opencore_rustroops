//! Hero layout math — welcome center, docked title-bar slot (layout A).

use crate::app::state::{HOME_WINDOW_HEIGHT, HOME_WINDOW_WIDTH};
use crate::app::viewport::WindowViewport;
use gpui_component::TITLE_BAR_HEIGHT;

pub const CUBE_HERO_LARGE: f32 = 220.0;
pub const CUBE_HERO_SMALL: f32 = 36.0;
pub const CUBE_HERO_MIN: f32 = 220.0;
pub const CUBE_HERO_MAX: f32 = 320.0;

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
    let width_limit = (available_width - WELCOME_EDGE_INSET_H * 2.0).max(CUBE_HERO_MIN);
    let height_limit = (available_height - WELCOME_ACTION_BAND).max(CUBE_HERO_MIN);
    width_limit.min(height_limit).min(CUBE_HERO_MAX)
}

/// Center of the large welcome cube in window coordinates.
pub fn welcome_cube_center(viewport: WindowViewport) -> (f32, f32) {
    let hero_size = responsive_hero_size(viewport.width, viewport.height);
    let content_top = WELCOME_HEADER_BAND;
    let content_height = (viewport.height - content_top - WELCOME_ACTION_BAND).max(hero_size);
    let center_y = content_top + content_height * 0.5;
    (viewport.width * 0.5, center_y)
}

/// Center of the docked cube: `[toggle-left] [cube] [wordmark]` (layout A).
pub fn docked_cube_center(viewport: WindowViewport) -> (f32, f32) {
    let title_h = TITLE_BAR_HEIGHT.as_f32();
    let x = title_bar_left_padding()
        + SHELL_TOGGLE_WIDTH
        + SHELL_TITLE_GAP
        + CUBE_HERO_SMALL * 0.5;
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
    fn docked_cube_sits_after_left_toggle() {
        let (x, y) = docked_cube_center(home_transition_viewport());
        let expected_x = title_bar_left_padding() + SHELL_TOGGLE_WIDTH + SHELL_TITLE_GAP + CUBE_HERO_SMALL * 0.5;
        assert!((x - expected_x).abs() < 1e-3);
        assert!((y - TITLE_BAR_HEIGHT.as_f32() * 0.5).abs() < 1e-3);
    }

    #[test]
    fn welcome_hero_size_clamps_on_narrow_windows() {
        assert_eq!(responsive_hero_size(440.0, 360.0), CUBE_HERO_MIN);
        assert_eq!(responsive_hero_size(1200.0, 900.0), CUBE_HERO_MAX);
    }
}
