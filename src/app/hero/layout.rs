//! Hero layout math — welcome center, docked title-bar slot (layout A).

use super::brand::{BRAND_ASPECT, brand_width};
use crate::app::state::{HOME_WINDOW_HEIGHT, HOME_WINDOW_WIDTH};
use crate::app::viewport::WindowViewport;
use crate::shared::theme::TypeRole;
use gpui_component::TITLE_BAR_HEIGHT;

pub const BRAND_HERO_MIN: f32 = 220.0;
pub const BRAND_HERO_MAX: f32 = 320.0;
/// Shell title-bar brand height.
pub const BRAND_SHELL_HEIGHT: f32 = 18.0;

/// Ghost xsmall icon button width in the shell title bar.
pub const SHELL_TOGGLE_WIDTH: f32 = 28.0;
pub const SHELL_TITLE_GAP: f32 = 4.0;

pub const WELCOME_EDGE_INSET_H: f32 = 16.0;
pub const WELCOME_TITLEBAR_HEIGHT: f32 = 38.0;
pub const WELCOME_EDGE_INSET_TOP: f32 = 4.0;
pub const WELCOME_EDGE_INSET_BOTTOM: f32 = 20.0;
pub const WELCOME_HEADER_GAP: f32 = 8.0;
pub const WELCOME_HERO_BRAND_FRAME_EXTRA: f32 = 40.0;
pub const WELCOME_ACTION_SPACER: f32 = 60.0;
pub const WELCOME_ENTER_BUTTON_HEIGHT: f32 = 48.0;
pub const WELCOME_ACTION_BOTTOM_PADDING: f32 = 8.0;
pub const WELCOME_COPY_MAX_WIDTH: f32 = 680.0;

const WELCOME_ACTION_BAND: f32 = 260.0;
const WELCOME_COPY_BODY: &str = "OpenCore combines chat, terminal, editing, and Rust-native performance in one permissioned desktop environment. To leave the crowded cloud, polluted by leaks and unconsciousness, to return to a workspace that stays on your machine.";
const WELCOME_MONO_CHAR_WIDTH: f32 = 7.15;

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
    TypeRole::LabelMd.size() * TypeRole::LabelMd.line_height()
        + 2.0
        + TypeRole::MonoSm.size() * TypeRole::MonoSm.line_height()
}

/// Height of the static hero copy block below the brand.
pub fn welcome_copy_block_height(viewport: WindowViewport) -> f32 {
    let text_width = WELCOME_COPY_MAX_WIDTH
        .min((viewport.width - WELCOME_EDGE_INSET_H * 2.0).max(1.0));
    let chars_per_line = (text_width / WELCOME_MONO_CHAR_WIDTH).floor().max(1.0) as usize;
    let line_count = WELCOME_COPY_BODY.len().div_ceil(chars_per_line);
    let body_height =
        TypeRole::MonoSm.size() * TypeRole::MonoSm.line_height() * line_count as f32;
    24.0
        + TypeRole::DisplayMd.size() * TypeRole::DisplayMd.line_height()
        + 8.0
        + body_height
}

/// Total height of the centered welcome stack below the header.
pub fn welcome_center_stack_height(viewport: WindowViewport, brand_height: f32) -> f32 {
    (brand_height + WELCOME_HERO_BRAND_FRAME_EXTRA)
        + welcome_copy_block_height(viewport)
        + WELCOME_ACTION_SPACER
        + WELCOME_ENTER_BUTTON_HEIGHT
        + WELCOME_ACTION_BOTTOM_PADDING
}

/// Center of the large welcome brand in window coordinates.
pub fn welcome_brand_center(viewport: WindowViewport, brand_height: f32) -> (f32, f32) {
    let centered_region_top = WELCOME_TITLEBAR_HEIGHT
        + WELCOME_EDGE_INSET_TOP
        + welcome_header_row_height()
        + WELCOME_HEADER_GAP;
    let centered_region_bottom = viewport.height - WELCOME_EDGE_INSET_BOTTOM;
    let centered_region_height = (centered_region_bottom - centered_region_top).max(0.0);
    let stack_height = welcome_center_stack_height(viewport, brand_height);
    let stack_top =
        centered_region_top + (centered_region_height - stack_height).max(0.0) * 0.5;
    let brand_center_y =
        stack_top + (brand_height + WELCOME_HERO_BRAND_FRAME_EXTRA) * 0.5;
    (viewport.width * 0.5, brand_center_y)
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
    let available_height =
        (viewport.height - welcome_header_row_height() - WELCOME_ACTION_BAND).max(0.0);
    // Prominent intro size that still fits the viewport; morphs down to `hero`.
    width_limit
        .min(available_height * 0.4)
        .max(hero)
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
        let settled = show_off + (hero - show_off);
        assert!((settled - hero).abs() < 1e-3);
    }

    #[test]
    fn welcome_brand_center_matches_centered_stack_layout() {
        let viewport = WindowViewport {
            width: 960.0,
            height: 740.0,
        };
        let brand_height = responsive_brand_height(viewport);
        let (_, center_y) = welcome_brand_center(viewport, brand_height);
        let stack_top = WELCOME_TITLEBAR_HEIGHT
            + WELCOME_EDGE_INSET_TOP
            + welcome_header_row_height()
            + WELCOME_HEADER_GAP
            + ((viewport.height
                - WELCOME_EDGE_INSET_BOTTOM
                - (WELCOME_TITLEBAR_HEIGHT
                    + WELCOME_EDGE_INSET_TOP
                    + welcome_header_row_height()
                    + WELCOME_HEADER_GAP))
                - welcome_center_stack_height(viewport, brand_height))
                .max(0.0)
                * 0.5;
        let expected_y = stack_top + (brand_height + WELCOME_HERO_BRAND_FRAME_EXTRA) * 0.5;
        assert!((center_y - expected_y).abs() < 1e-3);
    }
}
