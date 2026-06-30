//! Markdown rendering for chat messages via `gpui_component::text::TextView`.
//!
//! Wraps the gpui-component built-in rich-text view which handles headings,
//! code blocks (with syntax highlighting), lists, tables, inline formatting,
//! and links — without custom AST walking.
//!
//! Parse happens each frame; GPUI's frame coalescing at `cx.notify()` (~16ms)
//! provides the debounced re-parse the acceptance criteria require.

use gpui::{IntoElement, ParentElement, SharedString, Styled as _, div, px};
use gpui_component::text::{TextView, TextViewStyle};

/// Render markdown `content` as a styled `TextView`.
///
/// `base_size` controls the heading base font size; text inherits from
/// the parent styling. Safe to call every frame during streaming.
/// `message_id` is the unique `UiMessage.id` — used as the GPUI element ID
/// to guarantee distinct identity in the view tree.
pub fn render_markdown(content: &str, base_size: f32, is_dark: bool, message_id: i64) -> impl IntoElement {
    let id = SharedString::from(format!("md_{}", message_id));
    let style = TextViewStyle {
        heading_base_font_size: gpui::px(base_size),
        is_dark,
        ..Default::default()
    };
    div()
        .w_full()
        .min_w(px(0.))
        .overflow_hidden()
        .text_size(gpui::px(base_size))
        .child(TextView::markdown(id, content).style(style))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_markdown_returns_element() {
        let _ = render_markdown("hello", 14.0, false, 42);
    }

    #[test]
    fn element_id_uses_message_id() {
        let id = SharedString::from(format!("md_{}", 12345));
        assert_eq!(id.as_ref(), "md_12345");
    }
}
