//! Post-onboarding holy-grail chrome (Dock workspace + center titlebar tabs).

mod default_layout;
mod panels;
mod titlebar_tabs;
mod workspace;

pub use default_layout::{
    apply_default_holy_grail, BOTTOM_DEFAULT, DOCK_LAYOUT_VERSION, RIGHT_DEFAULT, SIDEBAR_DEFAULT,
};
pub use panels::{register_shell_panels, CenterStubHost};
pub use titlebar_tabs::{center_title_bar, render_center_tab_bar};
pub use workspace::{DockSaveFn, ShellWorkspace};
