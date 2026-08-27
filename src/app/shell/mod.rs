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

pub use default_layout::{
    BOTTOM_DEFAULT, DOCK_LAYOUT_VERSION, EDGE_DOCK_TAB_COUNT, RIGHT_DEFAULT, SIDEBAR_DEFAULT,
    apply_default_holy_grail, dock_item_enables_dnd, dock_item_panel_count,
};
pub use layout::ShellLayout;
pub use panels::register_shell_panels;
pub use workspace::{DockSaveFn, ShellWorkspace};
