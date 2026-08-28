//! Shared cube hero — welcome centerpiece and shell title-bar brand.

mod brand;
mod cube;
mod layout;
mod transition;

pub use brand::{BRAND_ASPECT, BRAND_IMAGE, BRAND_IMAGE_INVERSE, brand_width, opencore_brand_image};
pub use cube::{CubeHeroState, cube_hero_canvas};
pub use layout::{
    BRAND_SHELL_HEIGHT, CUBE_HERO_LARGE, CUBE_HERO_SMALL, docked_brand_center, docked_cube_center,
    responsive_brand_height, responsive_hero_size, title_bar_left_padding, welcome_brand_center,
    welcome_cube_center,
};
pub use transition::{HeroTransition, HERO_TRANSITION_DURATION};
