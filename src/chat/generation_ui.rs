//! Display helpers for composer generation controls.

use crate::api::ModelInfo;

pub fn model_unavailable_message(model_id: &str) -> String {
    format!(
        "Model \"{model_id}\" is not available. Choose another model from the catalog or refresh the list."
    )
}

pub fn catalog_loading_message() -> &'static str {
    "Model catalog is still loading. Wait for models to load or choose Auto."
}

pub fn effort_display_label(effort: &str) -> String {
    match effort {
        "max" => "Max".into(),
        "xhigh" => "X-High".into(),
        "high" => "High".into(),
        "medium" => "Medium".into(),
        "low" => "Low".into(),
        "minimal" => "Minimal".into(),
        "none" => "Off".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}

pub fn thinking_level_menu_options(model: &ModelInfo) -> Vec<(String, String)> {
    let Some(caps) = model.reasoning.as_ref() else {
        return Vec::new();
    };
    if caps.supported_efforts.is_empty() {
        return Vec::new();
    }

    let mut options = Vec::new();
    if !caps.mandatory {
        options.push(("default".into(), "Default".into()));
    }
    for effort in &caps.supported_efforts {
        options.push((effort.clone(), effort_display_label(effort)));
    }
    options
}

pub fn thinking_level_button_label(model: &ModelInfo, effort: &Option<String>) -> String {
    match effort.as_deref() {
        Some(value) => effort_display_label(value),
        None => model
            .reasoning
            .as_ref()
            .and_then(|caps| caps.default_effort.as_deref())
            .map(effort_display_label)
            .unwrap_or_else(|| "Thinking".into()),
    }
}

pub fn capability_lines(model: &ModelInfo) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(context) = model.context_length {
        lines.push(format!("{context} context window"));
    }
    if !model.input_modalities.is_empty() {
        lines.push(format!("Input: {}", model.input_modalities.join(", ")));
    }
    if !model.output_modalities.is_empty() {
        lines.push(format!("Output: {}", model.output_modalities.join(", ")));
    }
    if model.supports_thinking_controls()
        && let Some(caps) = model.reasoning.as_ref()
    {
        let labels: Vec<_> = caps
            .supported_efforts
            .iter()
            .map(|effort| effort_display_label(effort))
            .collect();
        lines.push(format!("Thinking levels: {}", labels.join(", ")));
    }
    if model.supports_speed_mode_controls() {
        lines.push("Speed mode supported".into());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ModelInfo, ReasoningCapabilities};

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
    fn thinking_level_menu_options_follow_catalog_efforts() {
        let model = codex_model();
        let options = thinking_level_menu_options(&model);
        assert_eq!(
            options.first().map(|(value, _)| value.as_str()),
            Some("default")
        );
        assert!(options.iter().any(|(value, _)| value == "none"));
        assert!(!options.iter().any(|(value, _)| value == "minimal"));
    }

    #[test]
    fn thinking_level_button_label_uses_model_default_effort() {
        let model = codex_model();
        assert_eq!(thinking_level_button_label(&model, &None), "Medium");
        assert_eq!(
            thinking_level_button_label(&model, &Some("low".into())),
            "Low"
        );
    }
}
