//! Normalized model catalog metadata and generation filtering.

use super::chat_provider::{GenerationSettings, SpeedMode};
use super::speed_mode::model_supports_speed_mode;

/// Normalized model metadata from a provider catalog.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_length: Option<u32>,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub supported_parameters: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningCapabilities>,
}

/// Provider-specific reasoning / thinking-level metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReasoningCapabilities {
    pub supported_efforts: Vec<String>,
    pub default_effort: Option<String>,
    pub mandatory: bool,
}

/// OpenRouter gateway efforts accepted when a model reports `supported_efforts: null`.
pub const GATEWAY_REASONING_EFFORTS: &[&str] =
    &["max", "xhigh", "high", "medium", "low", "minimal", "none"];

impl ModelInfo {
    pub fn supports_parameter(&self, parameter: &str) -> bool {
        self.supported_parameters
            .iter()
            .any(|value| value == parameter)
    }

    /// Router models pick a concrete provider at request time, so per-model
    /// generation metadata is not reliable for custom request fields.
    pub fn is_router_model(&self) -> bool {
        self.id == "openrouter/auto" || self.id.ends_with("/auto")
    }

    pub fn supports_temperature_controls(&self) -> bool {
        !self.is_router_model() && self.supports_parameter("temperature")
    }

    pub fn supports_max_tokens_controls(&self) -> bool {
        !self.is_router_model() && self.supports_parameter("max_tokens")
    }

    pub fn supports_thinking_controls(&self) -> bool {
        !self.is_router_model()
            && self.supports_parameter("reasoning")
            && self
                .reasoning
                .as_ref()
                .is_some_and(|caps| !caps.supported_efforts.is_empty())
    }

    pub fn is_supported_thinking_effort(&self, effort: &str) -> bool {
        self.reasoning
            .as_ref()
            .is_some_and(|caps| caps.supported_efforts.iter().any(|value| value == effort))
    }

    pub fn normalize_reasoning_effort(&self, effort: Option<String>) -> Option<String> {
        match effort {
            Some(value) if self.is_supported_thinking_effort(&value) => Some(value),
            Some(_) => None,
            None if self.supports_thinking_controls() => self
                .reasoning
                .as_ref()
                .and_then(|caps| {
                    if caps.mandatory {
                        caps.default_effort
                            .clone()
                            .filter(|effort| self.is_supported_thinking_effort(effort))
                    } else {
                        None
                    }
                }),
            None => None,
        }
    }

    /// Fast/normal speed presets (Anthropic `speed` or OpenAI `service_tier`).
    pub fn supports_speed_mode_controls(&self) -> bool {
        !self.is_router_model() && model_supports_speed_mode(&self.id)
    }

    pub fn filter_generation(&self, generation: &GenerationSettings) -> GenerationSettings {
        GenerationSettings {
            temperature: generation
                .temperature
                .filter(|_| self.supports_temperature_controls()),
            max_tokens: generation
                .max_tokens
                .filter(|_| self.supports_max_tokens_controls()),
            reasoning_effort: if self.supports_thinking_controls() {
                self.normalize_reasoning_effort(generation.reasoning_effort.clone())
            } else {
                None
            },
            speed_mode: if self.supports_speed_mode_controls() {
                generation.speed_mode
            } else {
                SpeedMode::Normal
            },
        }
    }

    pub fn sanitize_generation(&self, generation: &mut GenerationSettings) {
        *generation = self.filter_generation(generation);
    }
}

/// Generation settings safe to send when model metadata is unavailable.
pub fn generation_without_model_metadata(generation: &GenerationSettings) -> GenerationSettings {
    GenerationSettings {
        temperature: generation.temperature,
        max_tokens: generation.max_tokens,
        reasoning_effort: None,
        speed_mode: SpeedMode::Normal,
    }
}

/// Filters generation for a request, stripping model-specific fields when metadata is missing.
pub fn filter_generation_for_model(
    model: Option<&ModelInfo>,
    generation: &GenerationSettings,
) -> GenerationSettings {
    match model {
        Some(model) => model.filter_generation(generation),
        None => generation_without_model_metadata(generation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_model_filters_generation_controls() {
        let router = ModelInfo {
            id: "openrouter/auto".into(),
            name: "Auto Router".into(),
            context_length: Some(2_000_000),
            input_modalities: vec!["text".into()],
            output_modalities: vec!["text".into()],
            supported_parameters: vec![
                "temperature".into(),
                "max_tokens".into(),
                "reasoning".into(),
            ],
            reasoning: None,
        };
        assert!(router.is_router_model());
        assert!(!router.supports_temperature_controls());
        assert!(!router.supports_max_tokens_controls());
        assert!(!router.supports_thinking_controls());
        assert!(!router.supports_speed_mode_controls());

        let generation = GenerationSettings {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            reasoning_effort: Some("high".into()),
            speed_mode: SpeedMode::Fast,
        };
        assert_eq!(
            router.filter_generation(&generation),
            GenerationSettings::default()
        );
    }

    #[test]
    fn speed_mode_controls_match_opus_and_codex_families() {
        let opus = ModelInfo {
            id: "anthropic/claude-opus-4.8".into(),
            name: "Opus".into(),
            context_length: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            supported_parameters: vec!["reasoning".into()],
            reasoning: None,
        };
        let codex = ModelInfo {
            id: "openai/gpt-5.3-codex".into(),
            name: "Codex".into(),
            context_length: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            supported_parameters: vec!["reasoning".into()],
            reasoning: None,
        };
        let gpt = ModelInfo {
            id: "openai/gpt-5.5".into(),
            name: "GPT-5.5".into(),
            context_length: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            supported_parameters: vec!["reasoning".into()],
            reasoning: None,
        };
        assert!(opus.supports_speed_mode_controls());
        assert!(codex.supports_speed_mode_controls());
        assert!(gpt.supports_speed_mode_controls());
        assert!(
            !ModelInfo {
                id: "anthropic/claude-opus-4.8-fast".into(),
                name: "Opus Fast".into(),
                context_length: None,
                input_modalities: Vec::new(),
                output_modalities: Vec::new(),
                supported_parameters: Vec::new(),
                reasoning: None,
            }
            .supports_speed_mode_controls()
        );
    }

    #[test]
    fn thinking_controls_require_reasoning_parameter() {
        let model = ModelInfo {
            id: "provider/model".into(),
            name: "Model".into(),
            context_length: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            supported_parameters: vec!["reasoning_effort".into()],
            reasoning: None,
        };
        assert!(!model.supports_thinking_controls());
    }

    #[test]
    fn filter_generation_drops_unsupported_thinking_effort() {
        let model = ModelInfo {
            id: "anthropic/claude-opus-4.8".into(),
            name: "Opus".into(),
            context_length: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            supported_parameters: vec!["reasoning".into()],
            reasoning: Some(ReasoningCapabilities {
                supported_efforts: vec!["high".into(), "medium".into(), "low".into()],
                default_effort: Some("medium".into()),
                mandatory: false,
            }),
        };

        let filtered = model.filter_generation(&GenerationSettings {
            reasoning_effort: Some("minimal".into()),
            ..GenerationSettings::default()
        });
        assert_eq!(filtered.reasoning_effort, None);
    }

    #[test]
    fn mandatory_reasoning_uses_default_effort_when_unset() {
        let model = ModelInfo {
            id: "provider/model".into(),
            name: "Model".into(),
            context_length: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            supported_parameters: vec!["reasoning".into()],
            reasoning: Some(ReasoningCapabilities {
                supported_efforts: vec!["high".into(), "medium".into()],
                default_effort: Some("medium".into()),
                mandatory: true,
            }),
        };

        let filtered = model.filter_generation(&GenerationSettings::default());
        assert_eq!(filtered.reasoning_effort.as_deref(), Some("medium"));
    }

    #[test]
    fn generation_without_model_metadata_strips_speed_and_reasoning() {
        let generation = GenerationSettings {
            reasoning_effort: Some("high".into()),
            speed_mode: SpeedMode::Fast,
            ..GenerationSettings::default()
        };
        let stripped = generation_without_model_metadata(&generation);
        assert_eq!(stripped.reasoning_effort, None);
        assert_eq!(stripped.speed_mode, SpeedMode::Normal);
    }
}
