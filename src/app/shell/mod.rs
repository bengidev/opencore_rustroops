//! Post-onboarding holy-grail chrome (panels + main tabs).

mod chrome;
mod default_layout;
mod panels;
mod shell_view;
pub mod tabs;
pub mod tween;

pub use default_layout::{
    apply_default_holy_grail, BOTTOM_DEFAULT, DOCK_LAYOUT_VERSION, RIGHT_DEFAULT, SIDEBAR_DEFAULT,
};
pub use panels::register_shell_panels;
pub use chrome::{
    ShellChrome, ShellTabRecord, TITLEBAR_HEIGHT, clamp_bottom_height, clamp_right_width,
    clamp_sidebar_width,
};
pub use shell_view::{Shell, ShellSaveFn};
pub use tabs::TabModel;
pub use tween::{DimTween, RESIZE_MS, ease_out, eval_tween, tween_finished};
