//! Shared brand hero — welcome centerpiece and shell title-bar lockup.

mod brand;
mod layout;
mod transition;

pub use brand::{
    BRAND_ASPECT, BRAND_IMAGE, BRAND_IMAGE_INVERSE, brand_width, opencore_brand_image,
};
pub use layout::{
    BRAND_HERO_MAX, BRAND_HERO_MIN, BRAND_SHELL_HEIGHT, docked_brand_center,
    responsive_brand_height, responsive_hero_size, show_off_brand_height,
    title_bar_left_padding, welcome_brand_center,
};
pub use transition::{HERO_TRANSITION_DURATION, HeroTransition};
