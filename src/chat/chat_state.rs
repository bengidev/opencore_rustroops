//! In-memory chat state for the single implicit thread.

use crate::api::{MessageRole, ModelInfo};

use super::chat_store::ThreadSettings;

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
            return Ok(());
        }
        if self.models.iter().any(|model| model.id == model_id) {
            Ok(())
        } else {
            Err(format!(
                "Model \"{model_id}\" is not available. Choose another model from the catalog or refresh the list."
            ))
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
#[derive(Debug, Clone)]
pub struct ChatState {
    pub thread_id: Option<i64>,
    pub messages: Vec<UiMessage>,
    pub is_streaming: bool,
    pub error: Option<String>,
    pub thread_settings: ThreadSettings,
    pub catalog: ModelCatalogState,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            thread_id: None,
            messages: Vec::new(),
            is_streaming: false,
            error: None,
            thread_settings: ThreadSettings::default(),
            catalog: ModelCatalogState::default(),
        }
    }
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
}
