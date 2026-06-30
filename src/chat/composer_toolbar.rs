//! Compact composer controls for model and generation settings.

use gpui::{
    Anchor, Context, Hsla, IntoElement, ParentElement, SharedString, Styled, WeakEntity, div, px,
};
use gpui_component::Disableable;
use gpui_component::IconName;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::select::SelectState;
use gpui_component::select::SearchableVec;
use gpui_component::{Sizable, h_flex};

use crate::api::{GenerationSettings, ModelInfo};

use super::chat_view::ChatView;
use super::model_picker::{ModelSelectEntry, render_model_select};

pub const TEMPERATURE_OPTIONS: &[(Option<f32>, &str)] = &[
    (None, "Default"),
    (Some(0.3), "0.3"),
    (Some(0.5), "0.5"),
    (Some(0.7), "0.7"),
    (Some(1.0), "1.0"),
];

pub const MAX_TOKEN_OPTIONS: &[(Option<u32>, &str)] = &[
    (None, "Default"),
    (Some(1024), "1k"),
    (Some(2048), "2k"),
    (Some(4096), "4k"),
    (Some(8192), "8k"),
    (Some(16384), "16k"),
];

pub const REASONING_OPTIONS: &[(&str, &str)] = &[
    ("default", "Default"),
    ("high", "High"),
    ("medium", "Medium"),
    ("low", "Low"),
    ("minimal", "Minimal"),
    ("none", "Off"),
];

pub fn temperature_button_label(value: Option<f32>) -> SharedString {
    SharedString::from(match value {
        Some(v) => format!("{v:.1}"),
        None => "Temp".into(),
    })
}

pub fn max_tokens_button_label(value: Option<u32>) -> SharedString {
    SharedString::from(match value {
        None => "Tokens".into(),
        Some(v) if v >= 1024 && v % 1024 == 0 => format!("{}k", v / 1024),
        Some(v) => v.to_string(),
    })
}

pub fn reasoning_button_label(value: &Option<String>) -> SharedString {
    SharedString::from(match value.as_deref() {
        None | Some("default") => "Reasoning".into(),
        Some("high") => "High".into(),
        Some("medium") => "Medium".into(),
        Some("low") => "Low".into(),
        Some("minimal") => "Minimal".into(),
        Some("none") => "Off".into(),
        Some(other) => other.to_string(),
    })
}

pub fn capability_lines(model: &ModelInfo) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(context) = model.context_length {
        lines.push(format!("{context} context"));
    }
    if !model.input_modalities.is_empty() {
        lines.push(format!("Input: {}", model.input_modalities.join(", ")));
    }
    if !model.output_modalities.is_empty() {
        lines.push(format!("Output: {}", model.output_modalities.join(", ")));
    }
    if model.supports_reasoning_controls() {
        lines.push("Reasoning supported".into());
    }
    lines
}

pub fn render_composer_toolbar(
    model: Option<&ModelInfo>,
    generation: &GenerationSettings,
    model_select: &gpui::Entity<SelectState<SearchableVec<ModelSelectEntry>>>,
    catalog_refreshing: bool,
    muted: Hsla,
    border: Hsla,
    can_send: bool,
    cx: &mut Context<ChatView>,
) -> impl IntoElement {
    let weak = cx.entity().downgrade();
    let mut bar = h_flex()
        .w_full()
        .px(px(8.))
        .pb(px(8.))
        .pt(px(4.))
        .gap(px(2.))
        .items_center()
        .flex_wrap()
        .child(
            render_model_select(model_select)
                .appearance(false)
                .small()
                .menu_width(px(280.)),
        );

    if let Some(model) = model {
        if model.supports_temperature_controls() {
            bar = bar.child(toolbar_divider(border)).child(temperature_menu(
                weak.clone(),
                generation.temperature,
                muted,
            ));
        }
        if model.supports_max_tokens_controls() {
            bar = bar.child(toolbar_divider(border)).child(max_tokens_menu(
                weak.clone(),
                generation.max_tokens,
                muted,
            ));
        }
        if model.supports_reasoning_controls() {
            bar = bar.child(toolbar_divider(border)).child(reasoning_menu(
                weak.clone(),
                &generation.reasoning_effort,
                muted,
            ));
        }
        let lines = capability_lines(model);
        if !lines.is_empty() {
            bar = bar.child(toolbar_divider(border)).child(capabilities_menu(&lines, muted));
        }
    } else if catalog_refreshing {
        bar = bar.child(
            div()
                .text_size(px(11.))
                .text_color(muted)
                .child("Loading models…"),
        );
    }

    bar.child(div().flex_1().min_w(px(8.))).child(
        Button::new("send-message")
            .icon(IconName::ArrowUp)
            .primary()
            .small()
            .disabled(!can_send)
            .on_click(cx.listener(ChatView::on_send_clicked)),
    )
}

fn toolbar_divider(border: Hsla) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .w(px(1.))
        .h(px(14.))
        .mx(px(4.))
        .bg(border)
}

fn temperature_menu(
    view: WeakEntity<ChatView>,
    current: Option<f32>,
    muted: Hsla,
) -> impl IntoElement {
    let label = temperature_button_label(current);
    Button::new("temperature-menu")
        .ghost()
        .small()
        .text_color(muted)
        .label(label)
        .dropdown_caret(true)
        .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
            TEMPERATURE_OPTIONS.iter().fold(menu, |menu, (value, title)| {
                let checked = *value == current;
                let view = view.clone();
                let selected = *value;
                menu.item(
                    PopupMenuItem::new(SharedString::from(*title))
                        .checked(checked)
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |chat, cx| {
                                chat.set_temperature(selected, cx);
                            });
                        }),
                )
            })
        })
        .into_any_element()
}

fn max_tokens_menu(
    view: WeakEntity<ChatView>,
    current: Option<u32>,
    muted: Hsla,
) -> impl IntoElement {
    let label = max_tokens_button_label(current);
    Button::new("max-tokens-menu")
        .ghost()
        .small()
        .text_color(muted)
        .label(label)
        .dropdown_caret(true)
        .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
            MAX_TOKEN_OPTIONS.iter().fold(menu, |menu, (value, title)| {
                let checked = *value == current;
                let view = view.clone();
                let selected = *value;
                menu.item(
                    PopupMenuItem::new(SharedString::from(*title))
                        .checked(checked)
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |chat, cx| {
                                chat.set_max_tokens(selected, cx);
                            });
                        }),
                )
            })
        })
        .into_any_element()
}

fn reasoning_menu(
    view: WeakEntity<ChatView>,
    current: &Option<String>,
    muted: Hsla,
) -> impl IntoElement {
    let current = current.clone();
    let label = reasoning_button_label(&current);
    Button::new("reasoning-menu")
        .ghost()
        .small()
        .text_color(muted)
        .label(label)
        .dropdown_caret(true)
        .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
            REASONING_OPTIONS.iter().fold(menu, |menu, (value, title)| {
                let checked = match (current.as_deref(), *value) {
                    (None, "default") | (Some("default"), "default") => true,
                    (Some(effort), v) => effort == v,
                    _ => false,
                };
                let view = view.clone();
                let selected = *value;
                menu.item(
                    PopupMenuItem::new(SharedString::from(*title))
                        .checked(checked)
                        .on_click(move |_, _, cx| {
                            let _ = view.update(cx, |chat, cx| {
                                chat.set_reasoning_effort(selected, cx);
                            });
                        }),
                )
            })
        })
        .into_any_element()
}

fn capabilities_menu(lines: &[String], muted: Hsla) -> impl IntoElement {
    let lines = lines.to_vec();
    Button::new("model-capabilities")
        .ghost()
        .small()
        .icon(IconName::Info)
        .text_color(muted)
        .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
            lines.iter().fold(menu, |menu, line| {
                menu.item(PopupMenuItem::new(SharedString::from(line.clone())).disabled(true))
            })
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ModelInfo;

    #[test]
    fn compact_labels_use_short_forms() {
        assert_eq!(temperature_button_label(Some(0.7)).as_ref(), "0.7");
        assert_eq!(temperature_button_label(None).as_ref(), "Temp");
        assert_eq!(max_tokens_button_label(Some(4096)).as_ref(), "4k");
        assert_eq!(reasoning_button_label(&Some("low".into())).as_ref(), "Low");
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
        };
        let lines = capability_lines(&model);
        assert!(lines.iter().any(|line| line.contains("128000")));
        assert!(lines.iter().any(|line| line.starts_with("Input:")));
        assert!(lines.iter().any(|line| line.contains("Reasoning")));
    }

    #[test]
    fn capability_lines_omit_reasoning_for_router_models() {
        let router = ModelInfo {
            id: "openrouter/auto".into(),
            name: "Auto Router".into(),
            context_length: Some(2_000_000),
            input_modalities: vec!["text".into()],
            output_modalities: vec!["text".into()],
            supported_parameters: vec!["reasoning".into()],
        };
        let lines = capability_lines(&router);
        assert!(!lines.iter().any(|line| line.contains("Reasoning")));
    }
}
