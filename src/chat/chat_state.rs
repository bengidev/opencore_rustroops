//! In-memory chat state for the single implicit thread.

use crate::api::{ChatMessage, DEFAULT_MODEL, MessageRole, ModelInfo};

use super::chat_store::ThreadSettings;
use super::chat_store::ThreadInfo;
use super::generation_ui::{catalog_loading_message, model_unavailable_message};

/// Cached model catalog available to the chat UI.
#[derive(Debug, Clone, Default)]
pub struct ModelCatalogState {
    pub models: Vec<ModelInfo>,
    pub fetched_at: Option<String>,
    pub is_refreshing: bool,
}

impl ModelCatalogState {
    pub fn replace_catalog(&mut self, models: Vec<ModelInfo>, fetched_at: String) {
        self.models = models;
        self.fetched_at = Some(fetched_at);
        self.is_refreshing = false;
    }

    pub fn model_for_id(&self, model_id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|model| model.id == model_id)
    }

    pub fn validate_model_id(&self, model_id: &str) -> Result<(), String> {
        if self.models.is_empty() {
            if model_id == DEFAULT_MODEL {
                return Ok(());
            }
            return Err(catalog_loading_message().into());
        }
        if self.models.iter().any(|model| model.id == model_id) {
            Ok(())
        } else {
            Err(model_unavailable_message(model_id))
        }
    }
}

/// A message displayed in the chat UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMessage {
    pub id: i64,
    pub role: MessageRole,
    pub content: String,
}

/// Mutable chat state for one conversation thread.
#[derive(Debug, Clone, Default)]
pub struct ChatState {
    pub thread_id: Option<i64>,
    pub messages: Vec<UiMessage>,
    pub is_streaming: bool,
    pub error: Option<String>,
    pub thread_settings: ThreadSettings,
    pub threads: Vec<ThreadInfo>,
    pub catalog: ModelCatalogState,
}

impl ChatState {
    pub fn can_send(&self, credentials_missing: bool) -> bool {
        !credentials_missing && !self.is_streaming
    }

    pub fn append_user_message(&mut self, id: i64, content: String) {
        self.messages.push(UiMessage {
            id,
            role: MessageRole::User,
            content,
        });
        self.error = None;
    }

    pub fn begin_assistant_message(&mut self, id: i64) {
        self.messages.push(UiMessage {
            id,
            role: MessageRole::Assistant,
            content: String::new(),
        });
        self.is_streaming = true;
        self.error = None;
    }

    pub fn append_assistant_token(&mut self, message_id: i64, token: &str) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .find(|message| message.id == message_id)
        {
            message.content.push_str(token);
        }
    }

    pub fn finish_streaming(&mut self) {
        self.is_streaming = false;
    }

    pub fn set_error(&mut self, message: String) {
        self.error = Some(message);
        self.is_streaming = false;
    }

    /// Messages with non-empty content for provider API requests.
    ///
    /// Providers such as Cohere reject messages whose `content` is empty.
    pub fn api_messages(&self) -> Vec<ChatMessage> {
        self.messages
            .iter()
            .filter(|message| !message.content.trim().is_empty())
            .map(|message| ChatMessage {
                role: message.role,
                content: message.content.clone(),
            })
            .collect()
    }

    /// Drops trailing assistant placeholders left by cancelled or failed streams.
    pub fn remove_trailing_empty_assistants(&mut self) {
        while let Some(last) = self.messages.last() {
            if last.role == MessageRole::Assistant && last.content.trim().is_empty() {
                self.messages.pop();
            } else {
                break;
            }
        }
    }

    pub fn thread_title(&self) -> String {
        let Some(thread_id) = self.thread_id else {
            return "New Chat".into();
        };
        self.threads
            .iter()
            .find(|t| t.id == thread_id)
            .and_then(|t| t.title.clone())
            .unwrap_or_else(|| "New Chat".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_model_id_allows_default_model_while_catalog_empty() {
        let catalog = ModelCatalogState::default();
        assert!(catalog.validate_model_id(DEFAULT_MODEL).is_ok());
    }

    #[test]
    fn validate_model_id_blocks_unknown_models_while_catalog_empty() {
        let catalog = ModelCatalogState::default();
        assert!(
            catalog
                .validate_model_id("anthropic/claude-3.5-sonnet")
                .is_err()
        );
    }

    #[test]
    fn validate_model_id_checks_populated_catalog() {
        let catalog = ModelCatalogState {
            models: vec![ModelInfo {
                id: "provider/model".into(),
                name: "Model".into(),
                context_length: None,
                input_modalities: Vec::new(),
                output_modalities: Vec::new(),
                supported_parameters: Vec::new(),
                reasoning: None,
            }],
            ..ModelCatalogState::default()
        };
        assert!(catalog.validate_model_id("provider/model").is_ok());
        assert!(catalog.validate_model_id("missing/model").is_err());
    }

    #[test]
    fn api_messages_omits_empty_content() {
        let state = ChatState {
            messages: vec![
                UiMessage {
                    id: 1,
                    role: MessageRole::User,
                    content: "hello".into(),
                },
                UiMessage {
                    id: 2,
                    role: MessageRole::Assistant,
                    content: String::new(),
                },
                UiMessage {
                    id: 3,
                    role: MessageRole::User,
                    content: "again".into(),
                },
            ],
            ..ChatState::default()
        };

        let api = state.api_messages();
        assert_eq!(api.len(), 2);
        assert_eq!(api[0].content, "hello");
        assert_eq!(api[1].content, "again");
    }

    #[test]
    fn remove_trailing_empty_assistants_only_strips_suffix() {
        let mut state = ChatState {
            messages: vec![
                UiMessage {
                    id: 1,
                    role: MessageRole::User,
                    content: "hi".into(),
                },
                UiMessage {
                    id: 2,
                    role: MessageRole::Assistant,
                    content: String::new(),
                },
            ],
            ..ChatState::default()
        };

        state.remove_trailing_empty_assistants();
        assert_eq!(state.messages.len(), 1);
    }
}
