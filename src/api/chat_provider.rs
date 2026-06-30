//! Provider trait and shared request/response types.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::Stream;
use thiserror::Error;

/// Whether credentials are available and where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialStatus {
    Available { source: CredentialSource },
    Missing,
}

/// Origin of the active API credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    Environment,
    Saved,
}

/// Role of a message in a chat request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// A single message in a chat completion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// Per-request generation controls mapped to provider request fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenerationSettings {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub reasoning_effort: Option<String>,
}

/// Parameters for a streaming chat completion.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub generation: GenerationSettings,
}

/// Normalized model metadata from a provider catalog.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_length: Option<u32>,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub supported_parameters: Vec<String>,
}

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

    /// Reasoning UI maps to the OpenRouter `reasoning` request object.
    pub fn supports_reasoning_controls(&self) -> bool {
        !self.is_router_model() && self.supports_parameter("reasoning")
    }

    pub fn supports_reasoning(&self) -> bool {
        self.supports_reasoning_controls()
    }

    pub fn filter_generation(&self, generation: &GenerationSettings) -> GenerationSettings {
        GenerationSettings {
            temperature: generation
                .temperature
                .filter(|_| self.supports_temperature_controls()),
            max_tokens: generation
                .max_tokens
                .filter(|_| self.supports_max_tokens_controls()),
            reasoning_effort: generation
                .reasoning_effort
                .clone()
                .filter(|_| self.supports_reasoning_controls()),
        }
    }

    pub fn sanitize_generation(&self, generation: &mut GenerationSettings) {
        *generation = self.filter_generation(generation);
    }
}

/// Events emitted while streaming an assistant reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Token(String),
    Done,
}

/// Errors surfaced by provider implementations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ApiError {
    #[error("credentials are not configured")]
    MissingCredentials,
    #[error("model '{0}' is not available in the catalog")]
    UnknownModel(String),
    #[error("provider request failed: {0}")]
    RequestFailed(String),
    #[error("failed to parse provider response: {0}")]
    ParseError(String),
}

/// Cooperative cancellation handle for in-flight streams.
#[derive(Debug, Clone)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

pub type BoxedChatStream = Pin<Box<dyn Stream<Item = Result<StreamEvent, ApiError>> + Send>>;

pub type BoxedModelsFuture = Pin<Box<dyn Future<Output = Result<Vec<ModelInfo>, ApiError>> + Send>>;

/// Provider-agnostic chat boundary — the primary test seam for streaming behavior.
pub trait ChatProvider: Send + Sync {
    fn credential_status(&self) -> CredentialStatus;
    fn list_models(&self) -> BoxedModelsFuture;
    fn stream_chat(&self, request: ChatRequest, cancel: CancelToken) -> BoxedChatStream;
}

/// Accumulates token events from a stream into a single assistant reply string.
pub async fn accumulate_stream<S>(mut stream: S) -> Result<String, ApiError>
where
    S: Stream<Item = Result<StreamEvent, ApiError>> + Unpin,
{
    use futures::StreamExt;

    let mut content = String::new();
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::Token(token) => content.push_str(&token),
            StreamEvent::Done => break,
        }
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    struct FakeProvider {
        tokens: Vec<String>,
        status: CredentialStatus,
    }

    impl ChatProvider for FakeProvider {
        fn credential_status(&self) -> CredentialStatus {
            self.status.clone()
        }

        fn list_models(&self) -> BoxedModelsFuture {
            Box::pin(async { Ok(vec![]) })
        }

        fn stream_chat(&self, _request: ChatRequest, _cancel: CancelToken) -> BoxedChatStream {
            let tokens = self.tokens.clone();
            Box::pin(stream::iter(
                tokens
                    .into_iter()
                    .map(|token| Ok(StreamEvent::Token(token)))
                    .chain(std::iter::once(Ok(StreamEvent::Done))),
            ))
        }
    }

    #[test]
    fn cancel_token_starts_uncancelled() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

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
        };
        assert!(router.is_router_model());
        assert!(!router.supports_temperature_controls());
        assert!(!router.supports_max_tokens_controls());
        assert!(!router.supports_reasoning_controls());

        let generation = GenerationSettings {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            reasoning_effort: Some("high".into()),
        };
        assert_eq!(router.filter_generation(&generation), GenerationSettings::default());
    }

    #[test]
    fn reasoning_controls_require_reasoning_parameter() {
        let model = ModelInfo {
            id: "provider/model".into(),
            name: "Model".into(),
            context_length: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            supported_parameters: vec!["reasoning_effort".into()],
        };
        assert!(!model.supports_reasoning_controls());
    }

    #[test]
    fn accumulate_stream_joins_tokens_until_done() {
        futures::executor::block_on(async {
            let provider = FakeProvider {
                tokens: vec!["Hello".into(), ", ".into(), "world".into()],
                status: CredentialStatus::Missing,
            };
            let stream = provider.stream_chat(
                ChatRequest {
                    model: "test".into(),
                    messages: vec![],
                    generation: GenerationSettings::default(),
                },
                CancelToken::new(),
            );
            let content = accumulate_stream(stream).await.expect("accumulate");
            assert_eq!(content, "Hello, world");
        });
    }
}
