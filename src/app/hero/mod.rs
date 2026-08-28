//! Shared cube hero — welcome centerpiece and shell title-bar brand.

mod cube;
mod layout;
mod transition;

pub use cube::{CubeHeroState, cube_hero_canvas};
pub use layout::{
    CUBE_HERO_LARGE, CUBE_HERO_SMALL, docked_cube_center, responsive_hero_size, title_bar_left_padding,
    welcome_cube_center,
};
pub use transition::{HeroTransition, HERO_TRANSITION_DURATION};
