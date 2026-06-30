//! Capability chips and per-model generation controls.

use gpui::{App, AppContext, Entity, Hsla, IntoElement, ParentElement, Styled, Window, div, px};
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectState, SearchableVec};
use gpui_component::{Sizable, h_flex, v_flex};

use crate::api::{GenerationSettings, ModelInfo};
use crate::shared::theme::LegacyTypeRole;

const REASONING_EFFORTS: &[&str] = &[
    "max", "xhigh", "high", "medium", "low", "minimal", "none",
];

/// Input entities for adjustable generation parameters.
pub struct GenerationInputs {
    pub temperature: Entity<InputState>,
    pub max_tokens: Entity<InputState>,
    pub reasoning_effort: Entity<SelectState<SearchableVec<&'static str>>>,
}

impl GenerationInputs {
    pub fn new<C: 'static>(
        window: &mut Window,
        cx: &mut gpui::Context<C>,
        settings: &GenerationSettings,
    ) -> Self {
        let temperature = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("0.7")
                .default_value(settings.temperature.map(|value| value.to_string()).unwrap_or_default())
        });
        let max_tokens = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("4096")
                .default_value(
                    settings
                        .max_tokens
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                )
        });
        let selected_effort = settings
            .reasoning_effort
            .as_deref()
            .and_then(|effort| REASONING_EFFORTS.iter().position(|value| *value == effort))
            .map(|index| gpui_component::IndexPath::default().row(index));
        let reasoning_effort = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(REASONING_EFFORTS.to_vec()),
                selected_effort,
                window,
                cx,
            )
        });

        Self {
            temperature,
            max_tokens,
            reasoning_effort,
        }
    }

    pub fn read_settings(&self, cx: &App) -> GenerationSettings {
        GenerationSettings {
            temperature: parse_optional_f32(self.temperature.read(cx).value().trim()),
            max_tokens: parse_optional_u32(self.max_tokens.read(cx).value().trim()),
            reasoning_effort: self
                .reasoning_effort
                .read(cx)
                .selected_value()
                .map(|value| value.to_string()),
        }
    }
}

pub fn render_capability_chips(
    model: &ModelInfo,
    chip_bg: Hsla,
    chip_fg: Hsla,
    label: LegacyTypeRole,
) -> impl IntoElement {
    let text_size = px(label.size_px as f32 * 0.85);
    let mut chips = h_flex().gap(px(6.)).flex_wrap();

    if let Some(context) = model.context_length {
        chips = chips.child(chip(
            format!("{context} ctx"),
            chip_bg,
            chip_fg,
            text_size,
        ));
    }

    if !model.input_modalities.is_empty() {
        chips = chips.child(chip(
            format!("in: {}", model.input_modalities.join(", ")),
            chip_bg,
            chip_fg,
            text_size,
        ));
    }

    if !model.output_modalities.is_empty() {
        chips = chips.child(chip(
            format!("out: {}", model.output_modalities.join(", ")),
            chip_bg,
            chip_fg,
            text_size,
        ));
    }

    if model.supports_reasoning() {
        chips = chips.child(chip("reasoning".into(), chip_bg, chip_fg, text_size));
    }

    chips
}

pub fn render_generation_controls(
    model: &ModelInfo,
    inputs: &GenerationInputs,
    muted: Hsla,
    label: LegacyTypeRole,
) -> impl IntoElement {
    let text_size = px(label.size_px as f32 * 0.9);
    let mut controls = v_flex().gap(px(8.));

    if model.supports_parameter("temperature") {
        controls = controls.child(control_row(
            "Temperature",
            Input::new(&inputs.temperature).appearance(false),
            muted,
            text_size,
        ));
    }

    if model.supports_parameter("max_tokens") {
        controls = controls.child(control_row(
            "Max tokens",
            Input::new(&inputs.max_tokens).appearance(false),
            muted,
            text_size,
        ));
    }

    if model.supports_reasoning() {
        controls = controls.child(control_row(
            "Reasoning effort",
            Select::new(&inputs.reasoning_effort).small().placeholder("Default"),
            muted,
            text_size,
        ));
    }

    controls
}

fn chip(text: String, bg: Hsla, fg: Hsla, text_size: gpui::Pixels) -> impl IntoElement {
    div()
        .px(px(8.))
        .py(px(4.))
        .rounded_md()
        .bg(bg)
        .text_size(text_size)
        .text_color(fg)
        .child(text)
}

fn control_row(
    label: &str,
    control: impl IntoElement,
    muted: Hsla,
    text_size: gpui::Pixels,
) -> impl IntoElement {
    v_flex()
        .gap(px(4.))
        .child(
            div()
                .text_size(text_size)
                .text_color(muted)
                .child(label.to_string()),
        )
        .child(control)
}

fn parse_optional_f32(value: &str) -> Option<f32> {
    if value.is_empty() {
        None
    } else {
        value.parse().ok()
    }
}

fn parse_optional_u32(value: &str) -> Option<u32> {
    if value.is_empty() {
        None
    } else {
        value.parse().ok()
    }
}
