//! OpenRouter implementation of [`ChatProvider`].

use std::sync::Arc;

use reqwest::Client;
use serde::Deserialize;

use super::chat_provider::{
    ApiError, BoxedChatStream, BoxedModelsFuture, CancelToken, ChatProvider, ChatRequest,
    CredentialStatus, ModelInfo,
};
use super::credential_store::CredentialStore;
use super::credentials::{openrouter_credential_status, resolve_openrouter_api_key};
use super::http_runtime::spawn as spawn_http_task;
use super::openrouter_client::stream_chat_completion;

const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

/// Default model used until a per-thread selector ships in a later slice.
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
            spawn_http_task(async move {
                list_models_inner(credentials).await
            })
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
        let stream = stream_chat_completion(&api_key, &model, &messages, cancel);
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
}

async fn list_models_inner(
    credentials: Arc<dyn CredentialStore>,
) -> Result<Vec<ModelInfo>, ApiError> {
    let api_key =
        resolve_openrouter_api_key(credentials.as_ref()).ok_or(ApiError::MissingCredentials)?;
    let client = Client::builder()
        .build()
        .map_err(|error| ApiError::RequestFailed(error.to_string()))?;

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
        .map(|model| {
            let id = model.id.clone();
            ModelInfo {
                id,
                name: model.name.unwrap_or(model.id),
            }
        })
        .collect())
}
