//! In-memory chat state for the single implicit thread.

use crate::api::MessageRole;

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
        if let Some(message) = self.messages.iter_mut().find(|message| message.id == message_id) {
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
