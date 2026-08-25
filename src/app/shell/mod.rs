//! Post-welcome holy-grail chrome (Dock workspace + title bar toggles).

mod shell_default_layout;
mod shell_dock_animation;
mod shell_dock_tween;
mod shell_layout;
mod shell_panels;
mod shell_workspace;
mod workspace_theme;

pub use shell_default_layout::{
    BOTTOM_DEFAULT, DOCK_LAYOUT_VERSION, EDGE_DOCK_TAB_COUNT, RIGHT_DEFAULT, SIDEBAR_DEFAULT,
    apply_default_holy_grail, dock_item_enables_dnd, dock_item_panel_count,
};
pub use shell_layout::ShellLayout;
pub use shell_panels::register_shell_panels;
pub use shell_workspace::{DockSaveFn, ShellWorkspace};
