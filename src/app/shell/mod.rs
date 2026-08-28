//! Post-welcome holy-grail chrome (Dock workspace + title bar toggles).

mod default_layout;
mod dock_animation;
mod dock_tween;
mod layout;
mod left_sidebar;
mod main_workspace_panel;
mod panels;
mod workspace;
mod workspace_theme;

pub use default_layout::{DOCK_LAYOUT_VERSION, apply_default_holy_grail};
pub use panels::register_shell_panels;
pub use workspace::{DockSaveFn, ShellWorkspace};
