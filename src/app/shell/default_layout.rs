//! Default holy-grail Dock layout for the shell workspace.

use gpui::{App, AppContext, Edges, Entity, Window, px};
use gpui_component::dock::{DockArea, DockItem};

use super::panels::{BottomStubPanel, CenterStubHost, LeftStubPanel, RightStubPanel};

pub const DOCK_LAYOUT_VERSION: usize = 1;
pub const SIDEBAR_DEFAULT: f32 = 256.0;
pub const RIGHT_DEFAULT: f32 = 320.0;
pub const BOTTOM_DEFAULT: f32 = 220.0;

/// Applies the default holy-grail layout to `dock_area` and returns the center tab host.
pub fn apply_default_holy_grail(
    dock_area: &Entity<DockArea>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<CenterStubHost> {
    let weak = dock_area.downgrade();
    let center_host = cx.new(CenterStubHost::with_initial_tab);
    let center = DockItem::tab(center_host.clone(), &weak, window, cx);
    let left = DockItem::tab(cx.new(LeftStubPanel::new), &weak, window, cx);
    let right = DockItem::tab(cx.new(RightStubPanel::new), &weak, window, cx);
    let bottom = DockItem::tab(cx.new(BottomStubPanel::new), &weak, window, cx);

    dock_area.update(cx, |dock, cx| {
        dock.set_version(DOCK_LAYOUT_VERSION, window, cx);
        dock.set_center(center, window, cx);
        dock.set_left_dock(left, Some(px(SIDEBAR_DEFAULT)), true, window, cx);
        dock.set_right_dock(right, Some(px(RIGHT_DEFAULT)), false, window, cx);
        dock.set_bottom_dock(bottom, Some(px(BOTTOM_DEFAULT)), false, window, cx);
        dock.set_dock_collapsible(
            Edges {
                left: true,
                right: true,
                bottom: true,
                ..Default::default()
            },
            window,
            cx,
        );
    });
    center_host
}

#[cfg(test)]
mod tests {
    use super::{BOTTOM_DEFAULT, DOCK_LAYOUT_VERSION, RIGHT_DEFAULT, SIDEBAR_DEFAULT};

    #[test]
    fn default_layout_constants_match_spec() {
        assert_eq!(SIDEBAR_DEFAULT, 256.0);
        assert_eq!(RIGHT_DEFAULT, 320.0);
        assert_eq!(BOTTOM_DEFAULT, 220.0);
        assert_eq!(DOCK_LAYOUT_VERSION, 1);
    }
}
