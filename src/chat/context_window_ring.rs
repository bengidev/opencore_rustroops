//! Circular context-window usage ring for the composer footer.

use gpui::{Anchor, IntoElement, ParentElement, SharedString, Styled, div, px};
use gpui_component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::progress::ProgressCircle;
use gpui_component::{Sizable, Size};

use crate::api::ModelInfo;

use super::chat_state::UiMessage;
use super::composer_toolbar::{capability_lines, format_context_indicator};

/// Rough context usage from message character counts (~4 chars per token).
pub fn estimate_context_usage_percent(messages: &[UiMessage], context_length: u32) -> u8 {
    if context_length == 0 {
        return 0;
    }

    let char_count: usize = messages.iter().map(|message| message.content.chars().count()).sum();
    let estimated_tokens = char_count / 4;
    ((estimated_tokens as f64 / context_length as f64) * 100.0)
        .clamp(0.0, 100.0)
        .round() as u8
}

pub fn render_context_window_indicator(
    model: &ModelInfo,
    messages: &[UiMessage],
    muted: gpui::Hsla,
    _border: gpui::Hsla,
) -> impl IntoElement {
    let lines = capability_lines(model);
    let context_length = model.context_length;
    let usage_percent = context_length
        .map(|length| estimate_context_usage_percent(messages, length))
        .unwrap_or(0);
    let center_label = SharedString::from(usage_percent.to_string());
    let window_summary = context_length.map(|length| {
        SharedString::from(format!(
            "{} window · {usage_percent}% used (est.)",
            format_context_indicator(length)
        ))
    });

    let ring_size = px(24.);
    let ring = ProgressCircle::new("context-window-ring")
        .with_size(Size::Size(px(32.)))
        .value(usage_percent as f32)
        .color(muted)
        .child(
            div()
                .text_size(px(9.))
                .text_color(muted)
                .child(center_label),
        );

    div()
        .flex_shrink_0()
        .mr(px(4.))
        .child(
            Button::new("context-window-indicator")
                .ghost()
                .with_size(Size::Size(ring_size))
                .rounded(ButtonRounded::Size(ring_size * 0.5))
                .icon(ring)
                .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
                    let mut menu = menu;
                    if let Some(summary) = window_summary.clone() {
                        menu = menu.item(PopupMenuItem::new(summary).disabled(true));
                    }
                    if lines.is_empty() {
                        menu.item(PopupMenuItem::new("No model metadata").disabled(true))
                    } else {
                        lines.iter().fold(menu, |menu, line| {
                            menu.item(
                                PopupMenuItem::new(SharedString::from(line.clone())).disabled(true),
                            )
                        })
                    }
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_usage_is_zero_for_empty_thread() {
        assert_eq!(estimate_context_usage_percent(&[], 128_000), 0);
    }

    #[test]
    fn estimate_usage_scales_with_message_size() {
        let messages = vec![UiMessage {
            id: 1,
            role: crate::api::MessageRole::User,
            content: "x".repeat(40_000),
        }];
        let percent = estimate_context_usage_percent(&messages, 128_000);
        assert!(percent > 0);
        assert!(percent < 100);
    }

    #[test]
    fn estimate_usage_caps_at_one_hundred() {
        let messages = vec![UiMessage {
            id: 1,
            role: crate::api::MessageRole::User,
            content: "x".repeat(1_000_000),
        }];
        assert_eq!(estimate_context_usage_percent(&messages, 128_000), 100);
    }
}
