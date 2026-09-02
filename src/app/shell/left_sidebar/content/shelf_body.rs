//! Animated clip wrapper for collapsible shelf atom rows.

use gpui::{InteractiveElement, IntoElement, ParentElement, Styled, div, px};
use gpui_component::v_flex;

use super::super::tokens::SHELF_ROW_GAP;

pub fn sidebar_shelf_body(
    shelf_id: &str,
    clip_height: f32,
    show_content: bool,
    children: impl IntoIterator<Item = gpui::AnyElement>,
) -> gpui::AnyElement {
    if !show_content {
        return div().into_any_element();
    }

    v_flex()
        .id(format!("left-sidebar-shelf-body-{shelf_id}"))
        .w_full()
        .min_w_0()
        .gap(px(SHELF_ROW_GAP))
        .overflow_hidden()
        .h(px(clip_height.max(0.0)))
        .children(children)
        .into_any_element()
}
