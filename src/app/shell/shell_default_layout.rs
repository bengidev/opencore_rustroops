//! Default holy-grail Dock layout for the shell workspace.

use gpui::{App, AppContext, Edges, Entity, Window, px};
use gpui_component::dock::{DockArea, DockItem};

use super::left_sidebar::LeftSidebarPanel;
use super::main_workspace_panel::MainWorkspacePanel;
use super::shell_panels::{BottomStubPanel, FilesStubPanel, RightStubPanel};
use super::workspace_theme::workspace_theme;

pub const DOCK_LAYOUT_VERSION: usize = 9;
pub const SIDEBAR_DEFAULT: f32 = 256.0;
pub const RIGHT_DEFAULT: f32 = 320.0;
pub const BOTTOM_DEFAULT: f32 = 220.0;

/// Minimum panels per edge dock tab group in the default layout (demo variety).
/// Single-tab groups can still drag/merge via `PanelStyle::TabBar` and the
/// gpui-component draggable patch in `build.rs`.
pub const EDGE_DOCK_TAB_COUNT: usize = 2;

/// Wraps a tabs/tab item in a single-child `v_split`.
///
/// gpui-component locks any `TabPanel` with no `StackPanel` parent
/// (`stack_panel.is_none()` → not draggable/droppable). The story example
/// always nests tabs under a split; bare `DockItem::tabs` alone is not enough.
fn wrap_for_dnd(
    item: DockItem,
    dock_area: &gpui::WeakEntity<DockArea>,
    window: &mut Window,
    cx: &mut App,
) -> DockItem {
    DockItem::v_split(vec![item], dock_area, window, cx)
}

/// Applies the default holy-grail layout to `dock_area`.
///
/// Center uses native Dock tabs (`MainStubPanel`) so tabs live in the panel
/// tab bar and can join other tab groups via drag-and-drop.
pub fn apply_default_holy_grail(dock_area: &Entity<DockArea>, window: &mut Window, cx: &mut App) {
    let weak = dock_area.downgrade();
    let center = wrap_for_dnd(
        DockItem::tabs(
            vec![std::sync::Arc::new(cx.new(|cx| {
                MainWorkspacePanel::new(window, workspace_theme(), cx)
            }))],
            &weak,
            window,
            cx,
        ),
        &weak,
        window,
        cx,
    );
    let left = wrap_for_dnd(
        DockItem::tabs(
            vec![std::sync::Arc::new(cx.new(|cx| {
                LeftSidebarPanel::new(window, workspace_theme(), cx)
            }))],
            &weak,
            window,
            cx,
        ),
        &weak,
        window,
        cx,
    );
    let right = wrap_for_dnd(
        DockItem::tabs(
            vec![
                std::sync::Arc::new(cx.new(FilesStubPanel::new)),
                std::sync::Arc::new(cx.new(RightStubPanel::new)),
                std::sync::Arc::new(cx.new(|cx| RightStubPanel::with_title("OUTLINE", cx))),
            ],
            &weak,
            window,
            cx,
        ),
        &weak,
        window,
        cx,
    );
    let bottom = wrap_for_dnd(
        DockItem::tabs(
            vec![
                std::sync::Arc::new(cx.new(BottomStubPanel::new)),
                std::sync::Arc::new(cx.new(|cx| BottomStubPanel::with_title("TERMINAL", cx))),
            ],
            &weak,
            window,
            cx,
        ),
        &weak,
        window,
        cx,
    );

    dock_area.update(cx, |dock, cx| {
        dock.set_version(DOCK_LAYOUT_VERSION, window, cx);
        dock.set_center(center, window, cx);
        dock.set_left_dock(left, Some(px(SIDEBAR_DEFAULT)), false, window, cx);
        dock.set_right_dock(right, Some(px(RIGHT_DEFAULT)), false, window, cx);
        dock.set_bottom_dock(bottom, Some(px(BOTTOM_DEFAULT)), false, window, cx);
        dock.set_toggle_button_visible(false, cx);
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
}

/// Counts leaf panels under a dock item (tabs/split/panel).
pub fn dock_item_panel_count(item: &DockItem) -> usize {
    match item {
        DockItem::Tabs { items, .. } => items.len(),
        DockItem::Split { items, .. } => items.iter().map(dock_item_panel_count).sum(),
        DockItem::Panel { .. } => 1,
        DockItem::Tiles { items, .. } => items.len(),
    }
}

/// True when the item is a Split, which assigns TabPanels a `StackPanel` parent
/// so gpui-component unlocks drag-and-drop.
///
/// Bare `DockItem::tabs` / `tab` leave `TabPanel::stack_panel` as `None`, which
/// makes `is_locked()` true and disables both drag and drop.
pub fn dock_item_enables_dnd(item: &DockItem) -> bool {
    matches!(item, DockItem::Split { .. })
}

#[cfg(test)]
mod tests {
    use super::{
        BOTTOM_DEFAULT, DOCK_LAYOUT_VERSION, EDGE_DOCK_TAB_COUNT, RIGHT_DEFAULT, SIDEBAR_DEFAULT,
    };

    #[test]
    fn default_layout_constants_match_spec() {
        assert_eq!(SIDEBAR_DEFAULT, 256.0);
        assert_eq!(RIGHT_DEFAULT, 320.0);
        assert_eq!(BOTTOM_DEFAULT, 220.0);
        assert_eq!(DOCK_LAYOUT_VERSION, 9);
        assert_eq!(EDGE_DOCK_TAB_COUNT, 2);
    }
}
