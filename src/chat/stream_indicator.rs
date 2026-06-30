//! Loading and thinking indicators for assistant streaming states, plus
//! markdown rendering for completed and streaming assistant messages.

use std::time::Duration;

use super::markdown_render::render_markdown;
use gpui::{
    Animation, AnimationExt as _, Hsla, IntoElement, ParentElement, Styled as _, bounce, div,
    ease_in_out, px,
};
use gpui_component::h_flex;
use gpui_component::spinner::Spinner;
use gpui_component::{Icon, IconName, Sizable, Size};

use crate::api::MessageRole;

use super::chat_state::UiMessage;

/// Visual state for an in-flight assistant reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantStreamStatus {
    Idle,
    Thinking,
    Loading,
    Streaming,
}

pub fn assistant_stream_status(
    message: &UiMessage,
    is_streaming: bool,
    streaming_assistant_id: Option<i64>,
    supports_thinking: bool,
) -> AssistantStreamStatus {
    if !is_streaming || streaming_assistant_id != Some(message.id) {
        return AssistantStreamStatus::Idle;
    }
    if message.role != MessageRole::Assistant {
        return AssistantStreamStatus::Idle;
    }
    if message.content.trim().is_empty() {
        if supports_thinking {
            AssistantStreamStatus::Thinking
        } else {
            AssistantStreamStatus::Loading
        }
    } else {
        AssistantStreamStatus::Streaming
    }
}

pub fn render_assistant_stream_body(
    message: &UiMessage,
    status: AssistantStreamStatus,
    muted: Hsla,
    pill_bg: Hsla,
    label_size: f32,
    is_dark: bool,
) -> impl IntoElement {
    match status {
        AssistantStreamStatus::Thinking => div().child(render_status_pill(
            "Thinking",
            StreamPillStyle::Thinking,
            muted,
            pill_bg,
            label_size,
        )),
        AssistantStreamStatus::Loading => div().child(render_status_pill(
            "Loading",
            StreamPillStyle::Loading,
            muted,
            pill_bg,
            label_size,
        )),
        AssistantStreamStatus::Streaming => div().w_full().min_w(px(0.)).overflow_hidden().child(
            h_flex()
                .w_full()
                .min_w(px(0.))
                .overflow_hidden()
                .items_center()
                .gap(px(2.))
                .child(render_markdown(
                    &message.content,
                    label_size,
                    is_dark,
                    message.id,
                ))
                .child(render_streaming_cursor(muted)),
        ),
        AssistantStreamStatus::Idle => {
            div()
                .w_full()
                .min_w(px(0.))
                .overflow_hidden()
                .child(render_markdown(
                    &message.content,
                    label_size,
                    is_dark,
                    message.id,
                ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamPillStyle {
    Thinking,
    Loading,
}

fn render_status_pill(
    label: &'static str,
    style: StreamPillStyle,
    muted: Hsla,
    pill_bg: Hsla,
    label_size: f32,
) -> impl IntoElement {
    let mut pill = h_flex()
        .items_center()
        .gap(px(8.))
        .px(px(12.))
        .py(px(8.))
        .rounded_lg()
        .bg(pill_bg);

    pill = match style {
        StreamPillStyle::Thinking => pill.child(
            Icon::new(IconName::Bot)
                .with_size(Size::XSmall)
                .text_color(muted),
        ),
        StreamPillStyle::Loading => pill.child(Spinner::new().with_size(Size::XSmall).color(muted)),
    };

    pill.child(
        div()
            .text_size(px(label_size))
            .text_color(muted)
            .child(label),
    )
    .child(bouncing_dots(muted, label))
}

fn bouncing_dots(color: Hsla, id_prefix: &'static str) -> impl IntoElement {
    h_flex()
        .items_end()
        .gap(px(4.))
        .h(px(12.))
        .children((0..3).map(|index| bounce_dot(index, color, id_prefix)))
}

fn bounce_dot(index: usize, color: Hsla, id_prefix: &'static str) -> impl IntoElement {
    div()
        .w(px(4.5))
        .h(px(4.5))
        .rounded_full()
        .bg(color)
        .with_animation(
            (id_prefix, index),
            Animation::new(Duration::from_millis(900))
                .repeat()
                .with_easing(bounce(ease_in_out)),
            move |this, delta| {
                let phase = (delta + index as f32 * 0.22).fract();
                let lift = (phase * std::f32::consts::TAU).sin().max(0.0);
                this.opacity(0.35 + lift * 0.65).mt(px(-lift * 2.5))
            },
        )
}

fn render_streaming_cursor(muted: Hsla) -> impl IntoElement {
    div()
        .w(px(2.))
        .h(px(14.))
        .rounded_sm()
        .bg(muted)
        .with_animation(
            "streaming-cursor",
            Animation::new(Duration::from_millis(800))
                .repeat()
                .with_easing(ease_in_out),
            |this, delta| {
                let pulse = (delta * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                this.opacity(0.25 + pulse * 0.75)
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_assistant_while_streaming_is_thinking_for_reasoning_models() {
        let message = UiMessage {
            id: 7,
            role: MessageRole::Assistant,
            content: String::new(),
        };
        assert_eq!(
            assistant_stream_status(&message, true, Some(7), true),
            AssistantStreamStatus::Thinking
        );
    }

    #[test]
    fn empty_assistant_while_streaming_is_loading_without_reasoning() {
        let message = UiMessage {
            id: 7,
            role: MessageRole::Assistant,
            content: String::new(),
        };
        assert_eq!(
            assistant_stream_status(&message, true, Some(7), false),
            AssistantStreamStatus::Loading
        );
    }

    #[test]
    fn partial_assistant_content_is_streaming() {
        let message = UiMessage {
            id: 7,
            role: MessageRole::Assistant,
            content: "Hello".into(),
        };
        assert_eq!(
            assistant_stream_status(&message, true, Some(7), true),
            AssistantStreamStatus::Streaming
        );
    }
}
