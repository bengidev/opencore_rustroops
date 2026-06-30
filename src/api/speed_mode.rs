//! Speed-mode policy shared by composer capability checks and OpenRouter requests.

use super::chat_provider::SpeedMode;

/// OpenRouter request field used to request accelerated inference for a model family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedWireField {
    AnthropicFast,
    OpenAiPriority,
}

/// Returns the wire field to set when `mode` is [`SpeedMode::Fast`], if any.
pub fn speed_wire_field(model_id: &str, mode: SpeedMode) -> Option<SpeedWireField> {
    if mode != SpeedMode::Fast || model_id.ends_with("-fast") {
        return None;
    }

    if model_id == "anthropic/claude-opus-4.6"
        || model_id.starts_with("anthropic/claude-opus-4.7")
        || model_id.starts_with("anthropic/claude-opus-4.8")
    {
        return Some(SpeedWireField::AnthropicFast);
    }

    if model_id.starts_with("openai/gpt-5")
        && (model_id.contains("codex") || model_id.starts_with("openai/gpt-5.5"))
    {
        return Some(SpeedWireField::OpenAiPriority);
    }

    None
}

/// Whether a catalog model exposes fast/normal speed controls in the composer.
pub fn model_supports_speed_mode(model_id: &str) -> bool {
    speed_wire_field(model_id, SpeedMode::Fast).is_some()
}

/// Applies the speed-mode wire field for `model_id` to an OpenRouter request body.
pub fn apply_speed_to_json(body: &mut serde_json::Value, model_id: &str, mode: SpeedMode) {
    match speed_wire_field(model_id, mode) {
        Some(SpeedWireField::AnthropicFast) => {
            body["speed"] = serde_json::json!("fast");
        }
        Some(SpeedWireField::OpenAiPriority) => {
            body["service_tier"] = serde_json::json!("priority");
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SPEED_MODELS: &[&str] = &[
        "anthropic/claude-opus-4.6",
        "anthropic/claude-opus-4.8",
        "openai/gpt-5.3-codex",
        "openai/gpt-5.5",
    ];

    const NON_SPEED_MODELS: &[&str] = &[
        "anthropic/claude-opus-4.8-fast",
        "openai/gpt-4",
        "anthropic/claude-3.5-sonnet",
    ];

    #[test]
    fn speed_support_matches_wire_encoding() {
        for model_id in SPEED_MODELS {
            assert!(
                model_supports_speed_mode(model_id),
                "{model_id} should support speed controls"
            );
            assert!(
                speed_wire_field(model_id, SpeedMode::Fast).is_some(),
                "{model_id} should encode fast mode"
            );
        }
        for model_id in NON_SPEED_MODELS {
            assert!(
                !model_supports_speed_mode(model_id),
                "{model_id} should not support speed controls"
            );
            assert!(
                speed_wire_field(model_id, SpeedMode::Fast).is_none(),
                "{model_id} should not encode fast mode"
            );
        }
    }

    #[test]
    fn apply_speed_to_json_sets_expected_fields() {
        let mut anthropic = json!({});
        apply_speed_to_json(&mut anthropic, "anthropic/claude-opus-4.8", SpeedMode::Fast);
        assert_eq!(anthropic["speed"], "fast");

        let mut codex = json!({});
        apply_speed_to_json(&mut codex, "openai/gpt-5.3-codex", SpeedMode::Fast);
        assert_eq!(codex["service_tier"], "priority");

        let mut normal = json!({});
        apply_speed_to_json(&mut normal, "openai/gpt-5.3-codex", SpeedMode::Normal);
        assert!(normal.get("speed").is_none());
        assert!(normal.get("service_tier").is_none());
    }
}
