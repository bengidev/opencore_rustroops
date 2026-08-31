//! Shared brand hero — welcome centerpiece and shell title-bar lockup.

mod brand;
mod layout;

pub use brand::{
    BRAND_ASPECT, BRAND_IMAGE, BRAND_IMAGE_INVERSE, brand_width, opencore_brand_image,
};
pub use layout::{
    BRAND_HERO_MAX, BRAND_HERO_MIN, BRAND_SHELL_HEIGHT, WELCOME_ACTION_SPACER,
    WELCOME_EDGE_INSET_BOTTOM, WELCOME_EDGE_INSET_H, WELCOME_EDGE_INSET_TOP,
    WELCOME_ENTER_BUTTON_HEIGHT, WELCOME_HERO_BRAND_FRAME_EXTRA, WELCOME_TITLEBAR_HEIGHT, lerp_f32,
    responsive_brand_height, responsive_hero_size, show_off_brand_height, title_bar_left_padding,
    welcome_vertical_content_budget,
};
