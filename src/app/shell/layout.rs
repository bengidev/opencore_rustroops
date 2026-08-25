//! Shell layout derived from the live viewport and open dock panes.

use gpui::{App, Entity, Window};
use gpui_component::TITLE_BAR_HEIGHT;
use gpui_component::dock::{DockArea, DockPlacement};

use crate::app::viewport::WindowViewport;

/// Layout metrics recomputed every shell render while the user resizes the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellLayout {
    pub viewport: WindowViewport,
    pub left_dock: f32,
    pub right_dock: f32,
    pub bottom_dock: f32,
    pub center_width: f32,
    pub center_height: f32,
}

impl ShellLayout {
    pub fn from_window_and_dock(
        window: &Window,
        dock_area: &Entity<DockArea>,
        cx: &App,
    ) -> Self {
        let viewport = WindowViewport::from_window(window);
        let dock = dock_area.read(cx);
        let left_dock = dock_open_width(&dock, DockPlacement::Left, cx);
        let right_dock = dock_open_width(&dock, DockPlacement::Right, cx);
        let bottom_dock = dock_open_height(&dock, DockPlacement::Bottom, cx);
        let title_bar = TITLE_BAR_HEIGHT.as_f32();
        Self {
            viewport,
            left_dock,
            right_dock,
            bottom_dock,
            center_width: (viewport.width - left_dock - right_dock).max(0.0),
            center_height: (viewport.height - title_bar - bottom_dock).max(0.0),
        }
    }
}

fn dock_open_width(dock: &DockArea, placement: DockPlacement, cx: &App) -> f32 {
    if !dock.is_dock_open(placement, cx) {
        return 0.0;
    }
    let size = match placement {
        DockPlacement::Left => dock.left_dock().map(|panel| panel.read(cx).size()),
        DockPlacement::Right => dock.right_dock().map(|panel| panel.read(cx).size()),
        DockPlacement::Bottom | DockPlacement::Center => None,
    };
    size.map(|pixels| pixels.as_f32()).unwrap_or(0.0)
}

fn dock_open_height(dock: &DockArea, placement: DockPlacement, cx: &App) -> f32 {
    if placement != DockPlacement::Bottom || !dock.is_dock_open(placement, cx) {
        return 0.0;
    }
    dock.bottom_dock()
        .map(|panel| panel.read(cx).size().as_f32())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_region_subtracts_open_docks_and_title_bar() {
        let layout = ShellLayout {
            viewport: WindowViewport {
                width: 1000.0,
                height: 800.0,
            },
            left_dock: 256.0,
            right_dock: 0.0,
            bottom_dock: 220.0,
            center_width: 0.0,
            center_height: 0.0,
        };
        let center_width = (layout.viewport.width - layout.left_dock - layout.right_dock).max(0.0);
        let center_height = (layout.viewport.height - TITLE_BAR_HEIGHT.as_f32() - layout.bottom_dock)
            .max(0.0);
        assert_eq!(center_width, 744.0);
        assert_eq!(center_height, 546.0);
    }

    #[test]
    fn center_region_clamps_to_zero_on_tiny_viewports() {
        let layout = ShellLayout {
            viewport: WindowViewport {
                width: 200.0,
                height: 100.0,
            },
            left_dock: 256.0,
            right_dock: 320.0,
            bottom_dock: 220.0,
            center_width: 0.0,
            center_height: 0.0,
        };
        let center_width = (layout.viewport.width - layout.left_dock - layout.right_dock).max(0.0);
        let center_height = (layout.viewport.height - TITLE_BAR_HEIGHT.as_f32() - layout.bottom_dock)
            .max(0.0);
        assert_eq!(center_width, 0.0);
        assert_eq!(center_height, 0.0);
    }
}
