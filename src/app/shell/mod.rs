//! Post-onboarding holy-grail chrome (panels + main tabs).

mod chrome;
mod shell_view;
pub mod tabs;
pub mod tween;

pub use chrome::{
    BOTTOM_DEFAULT, BOTTOM_MAX_VH, BOTTOM_MIN, RIGHT_DEFAULT, RIGHT_MAX, RIGHT_MIN,
    SIDEBAR_DEFAULT, SIDEBAR_MAX, SIDEBAR_MIN, ShellChrome, ShellTabRecord, TITLEBAR_HEIGHT,
    clamp_bottom_height, clamp_right_width, clamp_sidebar_width,
};
pub use shell_view::{Shell, ShellSaveFn};
pub use tabs::TabModel;
pub use tween::{DimTween, RESIZE_MS, ease_out, eval_tween, tween_finished};
