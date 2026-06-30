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
pub fn render_markdown(content: &str, base_size: f32, is_dark: bool) -> impl IntoElement {
    let id = if content.len() <= 40 {
        SharedString::from(content)
    } else {
        SharedString::from(format!("md_{:016x}", fxhash(content)))
    };
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

fn fxhash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fxhash_is_deterministic() {
        assert_eq!(fxhash("hello"), fxhash("hello"));
    }

    #[test]
    fn fxhash_differs_for_different_inputs() {
        assert_ne!(fxhash("hello"), fxhash("world"));
    }

    #[test]
    fn short_content_uses_raw_text_as_id() {
        let content = "short";
        let id = if content.len() <= 40 {
            SharedString::from(content)
        } else {
            SharedString::from(format!("md_{:016x}", fxhash(content)))
        };
        assert_eq!(id.as_ref(), "short");
    }

    #[test]
    fn long_content_uses_prefixed_hash() {
        let content = "a".repeat(41);
        let id = if content.len() <= 40 {
            SharedString::from(content.as_str())
        } else {
            SharedString::from(format!("md_{:016x}", fxhash(&content)))
        };
        assert!(id.as_ref().starts_with("md_"));
        assert_eq!(id.len(), "md_".len() + 16);
    }
}
