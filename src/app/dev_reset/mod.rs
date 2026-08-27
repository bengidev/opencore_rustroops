//! Floating draggable debug reset overlay (debug-only rendering).

use std::rc::Rc;

use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Point, SharedString, Styled, Window, div, px,
};

use crate::app::gpui_callbacks::WindowAppHandler;
use crate::shared::theme::{BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme};

/// Click-vs-drag threshold in pixels (movement below this counts as a click).
pub(crate) const CLICK_THRESHOLD: f32 = 4.0;

/// FAB width for edge damping calculations.
pub(crate) const FAB_WIDTH: f32 = 80.0;

/// FAB height for edge damping calculations.
pub(crate) const FAB_HEIGHT: f32 = 28.0;

/// Bottom-right inset for default position.
const EDGE_INSET: f32 = 24.0;

/// Drag state for the dev reset FAB, owned by [`crate::app::desktop::OpenCoreApp`].
#[derive(Debug, Clone)]
pub struct DevResetState {
    /// Current FAB position in window pixels from top-left.
    pub origin: Point<Pixels>,
    /// Whether the user is currently dragging the FAB.
    pub dragging: bool,
    /// Where the mouse was when the drag started.
    pub pointer_start: Option<Point<Pixels>>,
    /// Where the FAB origin was when the drag started.
    pub origin_at_drag_start: Option<Point<Pixels>>,
}

impl Default for DevResetState {
    fn default() -> Self {
        // Bottom-right of the smaller (welcome) window: 960×740
        Self {
            origin: Point {
                x: px(960.0 - EDGE_INSET - FAB_WIDTH),
                y: px(740.0 - EDGE_INSET - FAB_HEIGHT),
            },
            dragging: false,
            pointer_start: None,
            origin_at_drag_start: None,
        }
    }
}

/// Returns true if mouse movement is under the click threshold (counts as click, not drag).
pub fn is_click_not_drag(dx: f32, dy: f32, threshold: f32) -> bool {
    dx.hypot(dy) < threshold
}

/// Dampens a proposed position past the [min, max] boundary.
///
/// Values within bounds pass through unchanged. Values past the boundary
/// are reduced (damped) so the element moves partway past the edge with
/// increasing resistance, never reaching the proposed overshoot.
pub fn damp_translation(_current: f32, min: f32, max: f32, proposed: f32) -> f32 {
    if proposed > max {
        let overshoot = proposed - max;
        max + overshoot * 0.5
    } else if proposed < min {
        let overshoot = min - proposed;
        min - overshoot * 0.5
    } else {
        proposed
    }
}

/// Handler for mouse down events on the FAB.
pub type MouseDownHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// Handler for mouse move events during drag.
pub type MouseMoveHandler = Rc<dyn Fn(&MouseMoveEvent, &mut Window, &mut App)>;

/// Handler for mouse up events (drag end / click detection).
pub type MouseUpHandler = Rc<dyn Fn(&MouseUpEvent, &mut Window, &mut App)>;

/// Callbacks for the dev reset overlay, constructed via [`DevResetCallbacks::from_app`].
#[derive(Clone)]
pub struct DevResetCallbacks {
    /// Called when the FAB is clicked without dragging.
    /// Wired to `reset_dev_data` on `OpenCoreApp`.
    #[allow(dead_code)]
    pub on_activate: WindowAppHandler,
    /// Called on mouse down on the FAB (starts drag tracking).
    pub on_drag_start: MouseDownHandler,
    /// Called on mouse move while dragging (updates FAB origin).
    pub on_drag_move: MouseMoveHandler,
    /// Called on mouse up (ends drag, checks click-vs-drag).
    pub on_drag_end: MouseUpHandler,
}

/// Renders the floating RESET FAB, absolutely positioned at `state.origin`.
///
/// Press feedback: opacity change (0.7) while `state.dragging` is true.
/// GPUI does not expose CSS `transform: scale()` for elements, so opacity
/// is used instead, per the brief's allowance.
pub fn dev_reset_fab(
    theme: OpenCoreTheme,
    state: &DevResetState,
    callbacks: &DevResetCallbacks,
) -> impl IntoElement {
    let surface = theme.surface(BackgroundToken::Tertiary);
    let foreground = theme.foreground(ForegroundToken::Primary);
    let border = theme.border_token(BorderToken::Strong);
    let on_drag_start = callbacks.on_drag_start.clone();

    div()
        .absolute()
        .left(state.origin.x)
        .top(state.origin.y)
        .px(px(12.))
        .py(px(6.))
        .border_1()
        .border_color(border)
        .bg(surface)
        .text_size(px(11.))
        .font_family(SharedString::from("Menlo"))
        .text_color(foreground)
        .cursor_pointer()
        .opacity(if state.dragging { 0.7 } else { 1.0 })
        .child("RESET")
        .on_mouse_down(
            MouseButton::Left,
            move |event: &MouseDownEvent, window, cx| {
                (on_drag_start)(event, window, cx);
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_movement_counts_as_click() {
        assert!(is_click_not_drag(2.0, 1.0, 4.0));
    }

    #[test]
    fn large_movement_counts_as_drag() {
        assert!(!is_click_not_drag(20.0, 0.0, 4.0));
    }

    #[test]
    fn damp_past_edge_reduces_delta() {
        // implement damp_translation(pos, min, max, proposed) and test
        assert!(damp_translation(10.0, 0.0, 100.0, 120.0) < 120.0);
        assert!(damp_translation(10.0, 0.0, 100.0, 120.0) > 100.0);
    }
}
