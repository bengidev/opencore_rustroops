//! Compact composer controls below the message input.

use gpui::{
    Anchor, Context, Entity, Hsla, IntoElement, ParentElement, SharedString, Styled, WeakEntity,
    div, px,
};
use gpui_component::Disableable;
use gpui_component::IconName;
use gpui_component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::select::SearchableVec;
use gpui_component::select::SelectState;
use gpui_component::spinner::Spinner;
use gpui_component::{Sizable, Size, h_flex};

use crate::api::{GenerationSettings, ModelInfo, SpeedMode};

use super::chat_state::UiMessage;
use super::composer_actions::ComposerActions;
use super::context_window_ring::render_context_window_indicator;
use super::generation_ui::{
    thinking_level_button_label, thinking_level_menu_options,
};
use super::model_picker::{ModelSelectEntry, render_composer_model_select};

pub const SPEED_MODE_OPTIONS: &[(SpeedMode, &str)] =
    &[(SpeedMode::Normal, "Normal"), (SpeedMode::Fast, "Fast")];

pub fn format_context_indicator(length: u32) -> String {
    if length >= 1_000_000 {
        format!("{}M", length / 1_000_000)
    } else if length >= 1024 && length.is_multiple_of(1024) {
        format!("{}k", length / 1024)
    } else if length >= 1000 {
        format!("{}k", length / 1000)
    } else {
        length.to_string()
    }
}

pub fn speed_mode_button_label(mode: SpeedMode) -> SharedString {
    SharedString::from(match mode {
        SpeedMode::Normal => "Normal",
        SpeedMode::Fast => "Fast",
    })
}

pub struct ComposerToolbarProps<'a> {
    pub model_select: &'a Entity<SelectState<SearchableVec<ModelSelectEntry>>>,
    pub model: Option<&'a ModelInfo>,
    pub messages: &'a [UiMessage],
    pub generation: &'a GenerationSettings,
    pub catalog_refreshing: bool,
    pub is_streaming: bool,
    pub muted: Hsla,
    pub border: Hsla,
    pub can_send: bool,
}

pub fn render_composer_toolbar<H: ComposerActions + 'static>(
    props: ComposerToolbarProps<'_>,
    cx: &mut Context<H>,
) -> impl IntoElement {
    let ComposerToolbarProps {
        model_select,
        model,
        messages,
        generation,
        catalog_refreshing,
        is_streaming,
        muted,
        border,
        can_send,
    } = props;
    let weak = cx.entity().downgrade();

    let mut bar = h_flex()
        .w_full()
        .h(px(36.))
        .px(px(10.))
        .items_center()
        .gap(px(2.))
        .border_t_1()
        .border_color(border)
        .child(render_composer_model_select(model_select));

    let mut needs_divider = true;

    if catalog_refreshing && model.is_none() {
        bar = bar.child(
            h_flex()
                .items_center()
                .gap(px(6.))
                .child(Spinner::new().with_size(Size::XSmall).color(muted))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(muted)
                        .child("Loading models…"),
                ),
        );
    }

    if let Some(model) = model {
        if model.supports_thinking_controls() {
            if needs_divider {
                bar = bar.child(toolbar_divider(border));
            }
            needs_divider = true;
            bar = bar.child(thinking_level_menu(
                weak.clone(),
                model,
                &generation.reasoning_effort,
                muted,
            ));
        }
        if model.supports_speed_mode_controls() {
            if needs_divider {
                bar = bar.child(toolbar_divider(border));
            }
            bar = bar.child(speed_mode_menu(weak.clone(), generation.speed_mode, muted));
        }
    }

    let send_weak = weak.clone();
    bar.child(div().flex_1().min_w(px(8.)))
        .children(
            model.map(|model| render_context_window_indicator(model, messages, muted, border)),
        )
        .child(if is_streaming {
            Button::new("send-message")
                .icon(Spinner::new().with_size(Size::XSmall).color(muted))
                .primary()
                .xsmall()
                .rounded(ButtonRounded::Size(px(14.)))
                .disabled(true)
        } else {
            Button::new("send-message")
                .icon(IconName::ArrowUp)
                .primary()
                .xsmall()
                .rounded(ButtonRounded::Size(px(14.)))
                .disabled(!can_send)
                .on_click(move |event, window, cx| {
                    let _ = send_weak.update(cx, |host, cx| {
                        host.on_send_clicked(event, window, cx);
                    });
                })
        })
}

fn toolbar_divider(border: Hsla) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .w(px(1.))
        .h(px(12.))
        .mx(px(2.))
        .bg(border)
}

fn compact_menu_button(id: &'static str, label: SharedString, muted: Hsla) -> Button {
    Button::new(id)
        .ghost()
        .xsmall()
        .text_color(muted)
        .label(label)
        .dropdown_caret(true)
}

fn speed_mode_menu<H: ComposerActions + 'static>(
    view: WeakEntity<H>,
    current: SpeedMode,
    muted: Hsla,
) -> impl IntoElement {
    let label = speed_mode_button_label(current);
    compact_menu_button("speed-mode-menu", label, muted)
        .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
            SPEED_MODE_OPTIONS
                .iter()
                .fold(menu, |menu, (value, title)| {
                    let checked = *value == current;
                    let view = view.clone();
                    let selected = *value;
                    menu.item(
                        PopupMenuItem::new(SharedString::from(*title))
                            .checked(checked)
                            .on_click(move |_, _, cx| {
                                let _ = view.update(cx, |host, cx| {
                                    host.set_speed_mode(selected, cx);
                                });
                            }),
                    )
                })
        })
        .into_any_element()
}

fn thinking_level_menu<H: ComposerActions + 'static>(
    view: WeakEntity<H>,
    model: &ModelInfo,
    current: &Option<String>,
    muted: Hsla,
) -> impl IntoElement {
    let options = thinking_level_menu_options(model);
    let current = current.clone();
    let label = SharedString::from(thinking_level_button_label(model, &current));
    compact_menu_button("thinking-level-menu", label, muted)
        .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
            options.iter().fold(menu, |menu, (value, title)| {
                let checked = match (current.as_deref(), value.as_str()) {
                    (None, "default") => true,
                    (Some(""), "default") => true,
                    (Some(effort), selected) => effort == selected,
                    _ => false,
                };
                let view = view.clone();
                let selected = value.clone();
                menu.item(
                    PopupMenuItem::new(SharedString::from(title.clone()))
                        .checked(checked)
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |host, cx| {
                                host.set_reasoning_effort(&selected, cx);
                            });
                        }),
                )
            })
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ModelInfo, ReasoningCapabilities};
    use crate::chat::generation_ui::capability_lines;

    #[test]
    fn format_context_indicator_uses_compact_suffixes() {
        assert_eq!(format_context_indicator(128_000), "125k");
        assert_eq!(format_context_indicator(2_000_000), "2M");
        assert_eq!(format_context_indicator(4096), "4k");
    }

    fn codex_model() -> ModelInfo {
        ModelInfo {
            id: "openai/gpt-5.3-codex".into(),
            name: "Codex".into(),
            context_length: Some(272_000),
            input_modalities: vec!["text".into()],
            output_modalities: vec!["text".into()],
            supported_parameters: vec!["reasoning".into()],
            reasoning: Some(ReasoningCapabilities {
                supported_efforts: vec![
                    "xhigh".into(),
                    "high".into(),
                    "medium".into(),
                    "low".into(),
                    "none".into(),
                ],
                default_effort: Some("medium".into()),
                mandatory: false,
            }),
        }
    }

    #[test]
    fn speed_mode_button_label_uses_fast_and_normal() {
        assert_eq!(
            speed_mode_button_label(SpeedMode::Normal),
            SharedString::from("Normal")
        );
        assert_eq!(
            speed_mode_button_label(SpeedMode::Fast),
            SharedString::from("Fast")
        );
    }

    #[test]
    fn capability_lines_summarize_model_metadata() {
        let model = ModelInfo {
            id: "test".into(),
            name: "Test".into(),
            context_length: Some(128_000),
            input_modalities: vec!["text".into(), "image".into()],
            output_modalities: vec!["text".into()],
            supported_parameters: vec!["reasoning".into()],
            reasoning: None,
        };
        let lines = capability_lines(&model);
        assert!(lines.iter().any(|line| line.contains("128000")));
        assert!(lines.iter().any(|line| line.starts_with("Input:")));
    }

    #[test]
    fn capability_lines_omit_thinking_for_router_models() {
        let router = ModelInfo {
            id: "openrouter/auto".into(),
            name: "Auto Router".into(),
            context_length: Some(2_000_000),
            input_modalities: vec!["text".into()],
            output_modalities: vec!["text".into()],
            supported_parameters: vec!["reasoning".into()],
            reasoning: None,
        };
        let lines = capability_lines(&router);
        assert!(!lines.iter().any(|line| line.contains("Thinking")));
        assert!(!lines.iter().any(|line| line.contains("Speed")));
    }

    #[test]
    fn capability_lines_include_speed_for_codex_models() {
        let codex = codex_model();
        let lines = capability_lines(&codex);
        assert!(lines.iter().any(|line| line.contains("Speed mode")));
        assert!(lines.iter().any(|line| line.contains("Thinking levels")));
    }
}
