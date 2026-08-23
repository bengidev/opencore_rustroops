//! Post-onboarding holy-grail chrome (Dock workspace + center titlebar tabs).

mod default_layout;
mod panels;
mod titlebar_tabs;
mod workspace;

pub use default_layout::{
    BOTTOM_DEFAULT, DOCK_LAYOUT_VERSION, RIGHT_DEFAULT, SIDEBAR_DEFAULT, apply_default_holy_grail,
};
pub use panels::{CenterStubHost, register_shell_panels};
pub use titlebar_tabs::{center_title_bar, render_center_tab_bar};
pub use workspace::{DockSaveFn, ShellWorkspace};
