//! OpenRouter implementation of [`ChatProvider`].

use std::sync::Arc;

use serde::Deserialize;

use super::chat_provider::{
    ApiError, BoxedChatStream, BoxedModelsFuture, CancelToken, ChatProvider, ChatRequest,
    CredentialStatus, GATEWAY_REASONING_EFFORTS, ModelInfo, ReasoningCapabilities,
};
use super::credential_store::CredentialStore;
use super::credentials::{openrouter_credential_status, resolve_openrouter_api_key};
use super::http_runtime::{http_client, spawn as spawn_http_task};
use super::openrouter_client::stream_chat_completion;

const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

/// Default model used when a thread has no saved selection.
pub const DEFAULT_MODEL: &str = "openrouter/auto";

/// OpenRouter-backed [`ChatProvider`].
pub struct OpenRouterProvider {
    credentials: Arc<dyn CredentialStore>,
}

impl OpenRouterProvider {
    pub fn new(credentials: Arc<dyn CredentialStore>) -> Self {
        Self { credentials }
    }
}

impl ChatProvider for OpenRouterProvider {
    fn credential_status(&self) -> CredentialStatus {
        openrouter_credential_status(self.credentials.as_ref())
    }

    fn list_models(&self) -> BoxedModelsFuture {
        let credentials = self.credentials.clone();
        Box::pin(async move {
            spawn_http_task(async move { list_models_inner(credentials).await })
                .await
                .map_err(|error| ApiError::RequestFailed(error.to_string()))?
        })
    }

    fn stream_chat(&self, request: ChatRequest, cancel: CancelToken) -> BoxedChatStream {
        let api_key = match resolve_openrouter_api_key(self.credentials.as_ref()) {
            Some(key) => key,
            None => {
                return Box::pin(futures::stream::once(async {
                    Err(ApiError::MissingCredentials)
                }));
            }
        };

        let model = request.model;
        let messages = request.messages;
        let generation = request.generation;
        let stream = stream_chat_completion(&api_key, &model, &messages, &generation, cancel);
        Box::pin(stream)
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<RemoteModel>,
}

#[derive(Debug, Deserialize)]
struct RemoteModel {
    id: String,
    name: Option<String>,
    context_length: Option<u32>,
    architecture: Option<RemoteArchitecture>,
    #[serde(default)]
    supported_parameters: Vec<String>,
    reasoning: Option<RemoteReasoning>,
}

#[derive(Debug, Deserialize)]
struct RemoteReasoning {
    #[serde(default)]
    mandatory: bool,
    supported_efforts: Option<Option<Vec<String>>>,
    default_effort: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RemoteArchitecture {
    #[serde(default)]
    input_modalities: Vec<String>,
    #[serde(default)]
    output_modalities: Vec<String>,
}

fn parse_reasoning_capabilities(remote: Option<RemoteReasoning>) -> Option<ReasoningCapabilities> {
    let remote = remote?;
    let supported_efforts = match remote.supported_efforts {
        None => return None,
        Some(None) => GATEWAY_REASONING_EFFORTS
            .iter()
            .map(|effort| (*effort).to_string())
            .collect(),
        Some(Some(efforts)) if efforts.is_empty() => return None,
        Some(Some(efforts)) => efforts,
    };

    Some(ReasoningCapabilities {
        supported_efforts,
        default_effort: remote.default_effort,
        mandatory: remote.mandatory,
    })
}

fn normalize_openrouter_model(model: RemoteModel) -> ModelInfo {
    let id = model.id.clone();
    let architecture = model.architecture.unwrap_or(RemoteArchitecture {
        input_modalities: Vec::new(),
        output_modalities: Vec::new(),
    });
    ModelInfo {
        id,
        name: model.name.unwrap_or(model.id),
        context_length: model.context_length,
        input_modalities: architecture.input_modalities,
        output_modalities: architecture.output_modalities,
        supported_parameters: model.supported_parameters,
        reasoning: parse_reasoning_capabilities(model.reasoning),
    }
}

async fn list_models_inner(
    credentials: Arc<dyn CredentialStore>,
) -> Result<Vec<ModelInfo>, ApiError> {
    let api_key =
        resolve_openrouter_api_key(credentials.as_ref()).ok_or(ApiError::MissingCredentials)?;
    let client = http_client();

    let response = client
        .get(OPENROUTER_MODELS_URL)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|error| ApiError::RequestFailed(error.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::RequestFailed(format!("HTTP {status}: {body}")));
    }

    let payload: ModelsResponse = response
        .json()
        .await
        .map_err(|error| ApiError::ParseError(error.to_string()))?;

    Ok(payload
        .data
        .into_iter()
        .map(normalize_openrouter_model)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_openrouter_model_maps_fixture_fields() {
        let fixture = include_str!("fixtures/openrouter_model.json");
        let remote: RemoteModel = serde_json::from_str(fixture).expect("parse fixture");
        let info = normalize_openrouter_model(remote);

        assert_eq!(info.id, "anthropic/claude-3.5-sonnet");
        assert_eq!(info.name, "Claude 3.5 Sonnet");
        assert_eq!(info.context_length, Some(200_000));
        assert_eq!(info.input_modalities, vec!["text", "image"]);
        assert_eq!(info.output_modalities, vec!["text"]);
        assert!(info.supports_parameter("temperature"));
        assert!(info.supports_parameter("max_tokens"));
        assert!(!info.supports_thinking_controls());
        assert!(!info.supports_parameter("structured_outputs"));
    }

    #[test]
    fn normalize_openrouter_model_maps_reasoning_efforts() {
        let remote: RemoteModel = serde_json::from_str(
            r#"{
                "id": "openai/gpt-5.3-codex",
                "name": "Codex",
                "supported_parameters": ["reasoning"],
                "reasoning": {
                    "mandatory": false,
                    "supported_efforts": ["xhigh", "high", "medium", "low", "none"],
                    "default_effort": "medium"
                }
            }"#,
        )
        .expect("parse model");

        let info = normalize_openrouter_model(remote);
        assert!(info.supports_thinking_controls());
        assert_eq!(
            info.thinking_level_menu_options()
                .iter()
                .map(|(value, _)| value.as_str())
                .collect::<Vec<_>>(),
            vec!["default", "xhigh", "high", "medium", "low", "none"]
        );
    }
}
