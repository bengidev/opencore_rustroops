//! Shared brand hero — welcome centerpiece and shell title-bar lockup.

mod brand;
mod layout;
mod transition;

pub use brand::{BRAND_ASPECT, BRAND_IMAGE, BRAND_IMAGE_INVERSE, brand_width, opencore_brand_image};
pub use layout::{
    BRAND_SHELL_HEIGHT, BRAND_HERO_MAX, BRAND_HERO_MIN, docked_brand_center,
    responsive_brand_height, responsive_hero_size, title_bar_left_padding, welcome_brand_center,
};
pub use transition::{HeroTransition, HERO_TRANSITION_DURATION};
